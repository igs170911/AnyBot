use anybot_context as _;
use anybot_domain as _;
use anybot_foundation as _;
use anybot_models as _;
use anybot_protocol as _;
use anybot_runner as _;
use anybot_store_client as _;
use anybot_tools as _;
use anybot_trust as _;
use proptest as _;
use serde_json as _;
use sha2 as _;
use test_harness as _;
use thiserror as _;
use tokio as _;

use std::sync::Arc;
use std::time::Duration;

use anybot_messages::{Lang, Params};
use anybot_protocol::{
    CardAnswer, ComputerPlatform, EvidenceKind, EvidencePayload, EvidenceRecord, KnownTaskEventKind,
};
use runner_test::turn::{Device, TurnWorld, WorldSetup};
use runner_test::turn_helpers::{
    fmt_text, messages_of, request_body, task_kinds, task_of, wait_for_shown_card,
};
use runner_test::turn_tool_helpers::{call, intent_call_ids, one_tool_step, tool_message_content};
use test_harness::timeout_real;

fn param_value(record: &EvidenceRecord, key: &str) -> Option<String> {
    let EvidencePayload::Rejected { reason } = &record.payload else {
        return None;
    };
    reason
        .params
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.clone())
}

fn rejected_of<'a>(kinds: &'a [KnownTaskEventKind], code: &str) -> Option<&'a EvidenceRecord> {
    for k in kinds {
        let KnownTaskEventKind::EvidenceAdded { record } = k else {
            continue;
        };
        if record.kind != EvidenceKind::Rejected {
            continue;
        }
        let EvidencePayload::Rejected { reason } = &record.payload else {
            continue;
        };
        if reason.code == code {
            return Some(record);
        }
    }
    None
}

async fn send_allowing_card(world: &Arc<TurnWorld>) -> Result<anybot_runner::TurnOutcome, String> {
    let shared = runner_test::turn::share(world);
    let mut bg = tokio::spawn(async move { shared.send(Device::Phone, "inspect").await });
    tokio::select! {
        joined = &mut bg => {
            return joined
                .map_err(|e| format!("join failed: {e}"))?
                .map_err(|e| format!("{e:?}"));
        }
        shown = wait_for_shown_card(world, 1) => {
            let (task, _cards) = shown?;
            world.answer_shown_card(task, CardAnswer::AllowOnce).await?;
        }
    }
    timeout_real(bg, Duration::from_secs(20))
        .await
        .map_err(|e| format!("turn timed out: {e}"))?
        .map_err(|e| format!("join failed: {e}"))?
        .map_err(|e| format!("{e:?}"))
}

const ANYBOT_DIR: &str = "/home/alice/.local/share/AnyBot";
const KEYS_PATH: &str = "/home/alice/.local/share/AnyBot/keys.db";

fn edit10_setup() -> WorldSetup {
    WorldSetup {
        platform: ComputerPlatform::Linux,
        computer_name: "Linux".to_owned(),
        workdir: "/home/alice".to_owned(),
        home_dir: "/home/alice".to_owned(),
        anybot_dir: ANYBOT_DIR.to_owned(),
        dirs: vec![
            "/home/alice/.local/share".to_owned(),
            "/home/alice/.local/share/notes".to_owned(),
            ANYBOT_DIR.to_owned(),
        ],
        files: vec![(KEYS_PATH.to_owned(), "KEYDB_SECRET".to_owned())],
        ..WorldSetup::default()
    }
}

fn assert_edit10_denied(
    kinds: &[KnownTaskEventKind],
    messages: &[serde_json::Value],
    list_id: &str,
) -> Result<(), String> {
    let started = kinds.iter().any(|k| {
        matches!(k, KnownTaskEventKind::ToolStarted { call_id, .. }
                 if call_id.as_str() != list_id)
    });
    if started {
        return Err("private read of the AnyBot folder must not start".to_owned());
    }
    let record = rejected_of(kinds, "gate-private-file").ok_or("missing private rejection")?;
    if param_value(record, "path").as_deref() != Some(KEYS_PATH) {
        return Err(format!(
            "wrong path param: {:?}",
            param_value(record, "path")
        ));
    }
    let expected = fmt_text(
        Lang::En,
        "gate-private-file",
        &Params::new().with_text("path", KEYS_PATH),
    )?;
    let refusal_found = messages
        .iter()
        .any(|m| m.get("content").and_then(|v| v.as_str()) == Some(expected.as_str()));
    if !refusal_found {
        return Err(format!("missing refusal text: {expected}"));
    }
    let body_text = serde_json::to_string(messages).map_err(|e| format!("{e:?}"))?;
    if body_text.contains("KEYDB_SECRET") {
        return Err("leaked AnyBot folder content".to_owned());
    }
    Ok(())
}

#[tokio::test]
async fn t_files_edit_10() -> Result<(), String> {
    let world = runner_test::turn::arc_world(TurnWorld::start(&edit10_setup()).await?);
    world.script(one_tool_step(vec![
        call("list_dir", "{\"path\":\"/home/alice/.local/share\"}", "c1"),
        call("read_file", &format!("{{\"path\":\"{KEYS_PATH}\"}}"), "c2"),
    ]));
    let outcome = send_allowing_card(&world).await?;
    let task = task_of(&outcome);
    let kinds = task_kinds(&world, &task);

    let ids = intent_call_ids(&world, &task);
    let list_id = ids.first().cloned().ok_or("no list call id")?;

    let body = request_body(&world.model, 1)?;
    let messages = messages_of(&body);
    let listing = tool_message_content(&messages, &list_id)?;
    if !listing.contains("notes/") {
        return Err(format!("listing lost the sibling entry: {listing}"));
    }
    if listing.contains("AnyBot/") {
        return Err(format!("listing must hide the AnyBot folder: {listing}"));
    }
    assert_edit10_denied(&kinds, &messages, &list_id)
}

#[tokio::test]
async fn t_files_n2_3() -> Result<(), String> {
    let setup = WorldSetup {
        platform: ComputerPlatform::BotMachine,
        computer_name: "bot-computer".to_owned(),
        bot_name: "Bot 1".to_owned(),
        workdir: "/home/bot/bot-1".to_owned(),
        dirs: vec!["/home/bot/bot-1".to_owned(), "/home/bot/bot-2".to_owned()],
        files: vec![(
            "/home/bot/bot-2/x.txt".to_owned(),
            "OTHER_BOT_SECRET".to_owned(),
        )],
        ..WorldSetup::default()
    };
    let world = TurnWorld::start(&setup).await?;
    world.script(one_tool_step(vec![call(
        "read_file",
        "{\"path\":\"/home/bot/bot-2/x.txt\"}",
        "c1",
    )]));
    let outcome = world.send(Device::Phone, "read other bot").await;
    let task = task_of(&outcome.map_err(|e| format!("{e:?}"))?);
    let kinds = task_kinds(&world, &task);

    let record = rejected_of(&kinds, "gate-bot-machine-outside").ok_or("missing rejection")?;
    if param_value(record, "bot").as_deref() != Some("Bot 1") {
        return Err(format!("wrong bot param: {:?}", param_value(record, "bot")));
    }
    if param_value(record, "path").as_deref() != Some("/home/bot/bot-2/x.txt") {
        return Err(format!(
            "wrong path param: {:?}",
            param_value(record, "path")
        ));
    }
    let expected = fmt_text(
        Lang::En,
        "gate-bot-machine-outside",
        &Params::new()
            .with_text("bot", "Bot 1")
            .with_text("path", "/home/bot/bot-2/x.txt"),
    )?;
    let body = request_body(&world.model, 1)?;
    let body_text = serde_json::to_string(&body).map_err(|e| format!("{e:?}"))?;
    if !body_text.contains(&expected) {
        return Err(format!("missing refusal text: {body_text}"));
    }
    if body_text.contains("OTHER_BOT_SECRET") {
        return Err("leaked other bot content".to_owned());
    }
    if !world.gated.logged().is_empty() {
        return Err(format!(
            "computer must not be touched: {:?}",
            world.gated.logged()
        ));
    }
    Ok(())
}

#[tokio::test]
async fn bot_reads_own_folder_directly() -> Result<(), String> {
    let setup = WorldSetup {
        platform: ComputerPlatform::BotMachine,
        computer_name: "bot-computer".to_owned(),
        bot_name: "Bot 1".to_owned(),
        workdir: "/home/bot/bot-1".to_owned(),
        dirs: vec!["/home/bot/bot-1".to_owned()],
        files: vec![(
            "/home/bot/bot-1/a.txt".to_owned(),
            "BOT_OWN_CONTENT".to_owned(),
        )],
        ..WorldSetup::default()
    };
    let world = TurnWorld::start(&setup).await?;
    world.script(one_tool_step(vec![call(
        "read_file",
        "{\"path\":\"/home/bot/bot-1/a.txt\"}",
        "c1",
    )]));
    let outcome = world
        .send(Device::Phone, "read own")
        .await
        .map_err(|e| format!("{e:?}"))?;
    let task = task_of(&outcome);
    if !outcome.pending.is_empty() {
        return Err(format!("expected no approval card: {:?}", outcome.pending));
    }
    let kinds = task_kinds(&world, &task);
    if rejected_of(&kinds, "gate-bot-machine-outside").is_some() {
        return Err("own folder read must not be rejected".to_owned());
    }
    let ids = intent_call_ids(&world, &task);
    let read_id = ids.first().cloned().ok_or("no read call id")?;
    let body = request_body(&world.model, 1)?;
    let messages = messages_of(&body);
    let got = tool_message_content(&messages, &read_id)?;
    if !got.contains("BOT_OWN_CONTENT") {
        return Err(format!("own folder content missing: {got}"));
    }
    Ok(())
}
