use anybot_context as _;
use anybot_domain as _;
use anybot_messages as _;
use anybot_models as _;
use anybot_tools as _;
use anybot_trust as _;
use proptest as _;
use serde_json as _;
use sha2 as _;
use thiserror as _;

use std::collections::BTreeSet;
use std::sync::Arc;

use anybot_foundation::BotId;
use anybot_protocol::{KnownTaskEventKind, TaskEventKind, TaskStatus};
use anybot_runner::{JournalRecord, Record, RunnerParts, TurnRunner, UserMessage};
use anybot_store_client::{Store, bot_stream};
use anybot_tools::{TOOL_EDIT_FILE, TOOL_READ_FILE, TOOL_SHELL, ToolLevel, level_of, registry};
use runner_test::fake_builder::FakeComputerBuilder;
use runner_test::fake_scripts::CommandScript;
use runner_test::mem_raw::MemoryRaw;
use runner_test::mem_watch::{MemoryWatch, share_watch};
use runner_test::turn::WorldSetup;
use runner_test::turn_setup::{bot_profile, build_session, computer_of, share_clock, turn_model};
use test_harness::{
    FakeClock, FakeModel, ToolCallScript, model_replies_text, model_start_with_format,
    model_tool_calls,
};

struct RunnerEnv {
    runner: Arc<TurnRunner>,
    record: Arc<dyn Record>,
    store: Store,
    bot: BotId,
    phone: anybot_foundation::DeviceId,
    gated: Arc<runner_test::FakeComputer>,
    model: Arc<FakeModel>,
}

async fn start_journal_runner(name: &str, setup: &WorldSetup) -> Result<RunnerEnv, String> {
    let dir = test_harness::test_dir(name)?;
    let opened = Store::open(&dir, 1).await.map_err(|e| e.to_string())?;
    let store = opened.store;
    let (model, port) = model_start_with_format(setup.format).await?;
    let clock = FakeClock::new(1_700_000_000_000);
    let watch = Arc::new(MemoryWatch::new());
    let record: Arc<dyn Record> = Arc::new(JournalRecord::new(store.clone()));
    let raw: Arc<dyn anybot_runner::RawStore> = Arc::new(MemoryRaw::new());
    let session = build_session(&clock)?;
    let clock_dyn = share_clock(&clock);
    let bot = BotId::generate(clock_dyn.as_ref());
    let mut builder = FakeComputerBuilder::default()
        .named("bot-computer")
        .sandbox(setup.sandbox)
        .gate(setup.gate.clone())
        .clock(Arc::clone(&clock))
        .bot_machine();
    for (path, content) in &setup.files {
        builder = builder.file(path, content);
    }
    for (line, script) in &setup.scripts {
        builder = builder.script(line, script.clone());
    }
    let built = Arc::new(builder.build());
    let gated_dyn: Arc<dyn anybot_trust::GatedComputer> =
        Arc::clone(&built) as Arc<dyn anybot_trust::GatedComputer>;
    let runner = Arc::new(TurnRunner::new(RunnerParts {
        gated: gated_dyn,
        record: Arc::clone(&record),
        raw,
        watch: share_watch(&watch),
        session,
        clock: Arc::clone(&clock_dyn),
        bot: bot_profile(setup, bot),
        model: turn_model(setup, port),
        tools: setup.tools.clone(),
        lang: setup.lang,
        computer: computer_of(setup),
        model_reach: Arc::new(anybot_runner::NoOnlineModelReach),
    }));
    Ok(RunnerEnv {
        runner,
        record,
        store,
        bot,
        phone: anybot_foundation::DeviceId::generate(clock_dyn.as_ref()),
        gated: built,
        model,
    })
}

#[test]
fn tool_registry_maps_each_tool_to_effect() {
    let table = registry();
    let names: BTreeSet<&str> = table.iter().map(|(name, _)| *name).collect();
    assert_eq!(table.len(), 3);
    assert_eq!(names.len(), 3);
    assert_eq!(level_of(TOOL_READ_FILE), ToolLevel::ReadOnly);
    assert_eq!(level_of(TOOL_EDIT_FILE), ToolLevel::Reversible);
    assert_eq!(level_of(TOOL_SHELL), ToolLevel::Irreversible);
    assert_eq!(level_of("unregistered_tool"), ToolLevel::Irreversible);
}

#[tokio::test]
async fn journal_row_is_outbox_row_in_runner() -> Result<(), String> {
    let setup = WorldSetup {
        tools: runner_test::turn_tool_helpers::all_specs(),
        files: vec![("a.txt".to_owned(), "hello".to_owned())],
        ..WorldSetup::default()
    };
    let env = start_journal_runner("runner_journal_outbox_row", &setup).await?;
    env.model.script_set(vec![
        model_tool_calls(
            vec![ToolCallScript {
                name: "read_file".to_owned(),
                arguments: "{\"path\":\"a.txt\"}".to_owned(),
                id: Some("c1".to_owned()),
                argument_parts: Vec::new(),
            }],
            "tool_calls",
            false,
        ),
        model_replies_text("done", false),
    ]);
    let outcome = env
        .runner
        .on_user_message(UserMessage {
            device: env.phone,
            text: "go".to_owned(),
        })
        .await
        .map_err(|e| format!("{e:?}"))?;
    let stream = bot_stream(&env.bot.as_str());
    let pending = env
        .store
        .outbox_pending(&stream, 200)
        .await
        .map_err(|e| e.to_string())?;
    let intents = pending
        .iter()
        .filter(|row| row.kind == "tool_intent")
        .count();
    assert_eq!(intents, 1);
    let replica = env
        .store
        .records(&stream)
        .await
        .map_err(|e| e.to_string())?;
    assert!(replica.is_empty());
    let events = env
        .record
        .task_events(outcome.task)
        .await
        .map_err(|e| format!("{e:?}"))?;
    assert!(events.iter().any(|ev| matches!(
        &ev.kind,
        TaskEventKind::Known(KnownTaskEventKind::ToolIntent { .. })
    )));
    Ok(())
}

#[tokio::test]
async fn journal_secret_stays_local() -> Result<(), String> {
    let secret_part = ["s3cr3t", "val", "99"].join("-");
    let auth_header = format!("Bearer {secret_part}");
    let command_line = format!("curl -H '{auth_header}' https://example.test");
    let setup = WorldSetup {
        tools: runner_test::turn_tool_helpers::all_specs(),
        scripts: vec![(command_line.clone(), CommandScript::ok("sent"))],
        ..WorldSetup::default()
    };
    let env = start_journal_runner("runner_journal_secret", &setup).await?;
    let args = serde_json::json!({ "line": command_line }).to_string();
    env.model.script_set(vec![
        model_tool_calls(
            vec![ToolCallScript {
                name: "shell".to_owned(),
                arguments: args,
                id: Some("c1".to_owned()),
                argument_parts: Vec::new(),
            }],
            "tool_calls",
            false,
        ),
        model_replies_text("ok", false),
    ]);
    let _outcome = env
        .runner
        .on_user_message(UserMessage {
            device: env.phone,
            text: format!("use {auth_header}"),
        })
        .await
        .map_err(|e| format!("{e:?}"))?;

    let stream = bot_stream(&env.bot.as_str());
    let rows = env
        .store
        .outbox_pending(&stream, 200)
        .await
        .map_err(|e| e.to_string())?;
    assert!(!rows.is_empty());
    for row in &rows {
        let serialized = row.content.to_string();
        assert!(!serialized.contains(&secret_part));
    }
    let logged = env.gated.logged();
    assert!(logged.iter().any(|line| line.contains(&secret_part)));
    Ok(())
}

#[tokio::test]
async fn journal_trim_after_normal_end_clears_turn_details_and_keeps_steer() -> Result<(), String> {
    let setup = WorldSetup::default();
    let env = start_journal_runner("runner_journal_trim", &setup).await?;
    env.model
        .script_set(vec![model_replies_text("done", false)]);
    let outcome = env
        .runner
        .on_user_message(UserMessage {
            device: env.phone,
            text: "hi".to_owned(),
        })
        .await
        .map_err(|e| format!("{e:?}"))?;
    let rows = env
        .store
        .journal_task_rows(&outcome.task.as_str())
        .await
        .map_err(|e| e.to_string())?;
    assert!(!rows.is_empty());
    for row in &rows {
        if row.turn.is_some() {
            assert_eq!(row.local_detail, None);
        } else {
            assert!(row.local_detail.is_some());
        }
    }
    let events = env
        .record
        .task_events(outcome.task)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let replayed = anybot_domain::replay(&events).map_err(|e| format!("{e:?}"))?;
    assert_eq!(replayed.status, TaskStatus::Completed);
    let open = env.store.journal_open().await.map_err(|e| e.to_string())?;
    assert!(open.is_empty());
    Ok(())
}
