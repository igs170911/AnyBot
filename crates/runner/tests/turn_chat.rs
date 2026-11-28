use anybot_context as _;
use anybot_domain as _;
use anybot_messages as _;
use anybot_models as _;
use anybot_runner as _;
use anybot_store_client as _;
use anybot_tools as _;
use anybot_trust as _;
use proptest as _;
use sha2 as _;
use thiserror as _;

use runner_test::turn::{Device, TurnWorld, WorldSetup};
use runner_test::turn_helpers::{
    chat_requests, kind_names, raw_bytes_of, sha256_hex, state_of, task_kinds, task_of,
};
use test_harness::{UsageScript, model_omit_usage, model_replies_text, model_usage};

const HELLO: &str = "\u{4f60}\u{597d}";
const DONE_CLAIM: &str = "\u{5b8c}\u{6210}\u{4e86}\u{3002}\u{8a30}\u{6458}\u{ff1a}\u{5df2}\u{9001}\u{51fa}\u{3000}3\u{527d}";

#[tokio::test]
async fn t_turn_loop_11() -> Result<(), String> {
    let setup = WorldSetup::default();
    let world = TurnWorld::start(&setup).await?;
    world.script(vec![model_replies_text(HELLO, false)]);
    let outcome = world.send(Device::Phone, HELLO).await;
    let outcome = outcome.map_err(|e| format!("{e:?}"))?;
    let task = task_of(&outcome);
    let state = state_of(&world, &task)?;
    assert_eq!(state.0, anybot_protocol::TaskStatus::Completed);
    assert_eq!(state.1, Some(anybot_protocol::TaskDetail::Replied));
    assert_eq!(
        kind_names(&world, &task),
        [
            "task_created",
            "turn_started",
            "context_built",
            "model_asked",
            "model_answer",
            "model_usage",
            "budget_usage",
            "turn_ended"
        ]
    );
    assert_eq!(chat_requests(&world.model).len(), 1);
    let seqs: Vec<u64> = world.task_events(task).iter().map(|e| e.seq).collect();
    assert_eq!(seqs, (1u64..=8).collect::<Vec<u64>>());
    check_answer_and_end(&world, &task)?;
    Ok(())
}

fn check_answer_and_end(world: &TurnWorld, task: &anybot_foundation::TaskId) -> Result<(), String> {
    let kinds = task_kinds(world, task);
    let Some(anybot_protocol::KnownTaskEventKind::TurnStarted { initiator, .. }) = kinds.get(1)
    else {
        return Err("no turn_started".to_owned());
    };
    assert_eq!(
        *initiator,
        anybot_protocol::TurnInitiator::You {
            device: world.device(Device::Phone)
        }
    );
    let Some(anybot_protocol::KnownTaskEventKind::ModelAnswer {
        text,
        tool_requests,
        ..
    }) = kinds.get(4)
    else {
        return Err("no model_answer".to_owned());
    };
    assert_eq!(text, HELLO);
    assert_eq!(tool_requests.len(), 0);
    let Some(anybot_protocol::KnownTaskEventKind::TurnEnded {
        reason,
        reply_delivered,
    }) = kinds.get(7)
    else {
        return Err("no turn_ended".to_owned());
    };
    assert_eq!(*reason, anybot_protocol::TurnEndReason::Answered);
    assert!(*reply_delivered);
    Ok(())
}

#[tokio::test]
async fn turn_reply_claiming_done_changes_nothing() -> Result<(), String> {
    let setup = WorldSetup::default();
    let claim = TurnWorld::start(&setup).await?;
    claim.script(vec![model_replies_text(DONE_CLAIM, false)]);
    let plain = TurnWorld::start(&setup).await?;
    plain.script(vec![model_replies_text("ok", false)]);
    let first = claim.send(Device::Phone, "go").await;
    let second = plain.send(Device::Phone, "go").await;
    let first = first.map_err(|e| format!("{e:?}"))?;
    let second = second.map_err(|e| format!("{e:?}"))?;
    let left = state_of(&claim, &task_of(&first))?;
    let right = state_of(&plain, &task_of(&second))?;
    assert_eq!(left, right);
    assert_eq!(
        kind_names(&claim, &task_of(&first)),
        kind_names(&plain, &task_of(&second))
    );
    assert!(!kind_names(&claim, &task_of(&first)).contains(&"evidence_added".to_owned()));
    Ok(())
}

#[tokio::test]
async fn turn_usage_absent_and_present() -> Result<(), String> {
    let setup = WorldSetup::default();
    let world = TurnWorld::start(&setup).await?;
    world.script(vec![
        model_usage("hi", UsageScript::plain(12, 3), false),
        model_omit_usage("hi"),
    ]);
    let first = world.send(Device::Phone, "one").await;
    let second = world.send(Device::Phone, "two").await;
    let first = first.map_err(|e| format!("{e:?}"))?;
    let second = second.map_err(|e| format!("{e:?}"))?;
    check_usage(&world, &task_of(&first), 12, 3)?;
    check_usage(&world, &task_of(&second), 0, 0)?;
    check_manifest(&world, &task_of(&first))?;
    check_manifest(&world, &task_of(&second))?;
    Ok(())
}

fn check_usage(
    world: &TurnWorld,
    task: &anybot_foundation::TaskId,
    input: u64,
    output: u64,
) -> Result<(), String> {
    let kinds = task_kinds(world, task);
    let Some(anybot_protocol::KnownTaskEventKind::ModelUsage { usage, .. }) =
        find_kind(&kinds, "model_usage")
    else {
        return Err("no model_usage".to_owned());
    };
    if input == 0 && output == 0 {
        assert_eq!(usage.clone(), None);
        return Ok(());
    }
    let Some(got) = *usage else {
        return Err("usage absent".to_owned());
    };
    assert_eq!(
        got,
        anybot_protocol::TokenUsage {
            input,
            cache_read: 0,
            cache_write: 0,
            output,
            thinking: 0,
        }
    );
    Ok(())
}

fn find_kind<'a>(
    kinds: &'a [anybot_protocol::KnownTaskEventKind],
    name: &str,
) -> Option<&'a anybot_protocol::KnownTaskEventKind> {
    kinds.iter().find(|k| k.kind_name() == name)
}

fn check_manifest(world: &TurnWorld, task: &anybot_foundation::TaskId) -> Result<(), String> {
    let kinds = task_kinds(world, task);
    let Some(anybot_protocol::KnownTaskEventKind::ContextBuilt { manifest_id, .. }) =
        find_kind(&kinds, "context_built")
    else {
        return Err("no context_built".to_owned());
    };
    let Some(anybot_protocol::KnownTaskEventKind::ModelUsage {
        manifest_id: usage_id,
        ..
    }) = find_kind(&kinds, "model_usage")
    else {
        return Err("no model_usage".to_owned());
    };
    assert_eq!(manifest_id, usage_id);
    let bytes = raw_bytes_of(&world.raw, manifest_id).ok_or("manifest missing".to_owned())?;
    assert_eq!(sha256_hex(&bytes), *manifest_id);
    let doc: serde_json::Value = serde_json::from_slice(&bytes).map_err(|e| format!("{e}"))?;
    assert!(doc.get("budget").is_some());
    Ok(())
}

use anybot_context as _;
use anybot_domain as _;
use anybot_messages as _;
use sha2 as _;
