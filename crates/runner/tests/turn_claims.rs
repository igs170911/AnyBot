use anybot_context as _;
use anybot_domain as _;
use anybot_foundation as _;
use anybot_messages as _;
use anybot_models as _;
use anybot_protocol as _;
use anybot_runner as _;
use anybot_tools as _;
use anybot_trust as _;
use proptest as _;
use serde_json as _;
use sha2 as _;
use test_harness as _;
use thiserror as _;

use std::sync::Arc;
use std::time::Duration;

use anybot_protocol::KnownTaskEventKind;
use anybot_runner::StoredBody;
use anybot_trust::{CODE_CLAIM_CHAT_DEAD_LINK, CODE_CLAIM_CHAT_STEP_FAILED};
use runner_test::fake_scripts::CommandScript;
use runner_test::turn::{Device, TurnWorld, WorldSetup};
use runner_test::turn_helpers::{chat_requests, drive, kind_names, task_kinds, task_of};
use runner_test::turn_tool_helpers::{all_specs, call};
use test_harness::{advance_clock, model_accept_then_delay, model_replies_text, model_tool_calls};

fn created_task(world: &TurnWorld) -> Option<anybot_foundation::TaskId> {
    for row in world.record.snapshot() {
        let StoredBody::Task { task, event, .. } = &row.body else {
            continue;
        };
        if event.seq == 1 {
            return Some(*task);
        }
    }
    None
}

async fn wait_for_request(world: &TurnWorld, count: usize) -> Result<(), String> {
    for _step in 0..400 {
        if chat_requests(&world.model).len() >= count {
            return Ok(());
        }
        advance_clock(&world.clock, Duration::from_millis(10));
        tokio::task::yield_now().await;
    }
    Err("request never arrived".to_owned())
}

async fn run_shared(
    world: Arc<TurnWorld>,
    from: Device,
    text: &str,
) -> Result<anybot_runner::TurnOutcome, anybot_runner::TurnError> {
    world.send(from, text).await
}

#[tokio::test]
async fn t_claims_check_5() -> Result<(), String> {
    let setup = WorldSetup {
        tools: all_specs(),
        scripts: vec![("grep x a.txt".to_owned(), CommandScript::failing("", 1))],
        ..WorldSetup::default()
    };
    let world = TurnWorld::start(&setup).await?;
    world.script(vec![
        model_tool_calls(
            vec![call("shell", "{\"line\":\"grep x a.txt\"}", "c1")],
            "tool_calls",
            false,
        ),
        model_replies_text(
            "\u{505a}\u{597d}\u{4e86}\u{ff0c}\u{6c92}\u{6709}\u{627e}\u{5230}",
            false,
        ),
    ]);
    let outcome = world
        .send(Device::Phone, "find x")
        .await
        .map_err(|e| format!("{e:?}"))?;
    let task = task_of(&outcome);
    assert!(!kind_names(&world, &task).contains(&"claim_checked".to_owned()));
    Ok(())
}

#[tokio::test]
async fn t_claims_check_7() -> Result<(), String> {
    let setup = WorldSetup {
        tools: all_specs(),
        scripts: vec![("build-step".to_owned(), CommandScript::failing("err", 2))],
        ..WorldSetup::default()
    };
    let world = Arc::new(TurnWorld::start(&setup).await?);
    world.script(vec![
        model_tool_calls(
            vec![call("shell", "{\"line\":\"build-step\"}", "c1")],
            "tool_calls",
            false,
        ),
        model_accept_then_delay(5, model_replies_text("\u{5df2}\u{5b8c}\u{6210}", false)),
    ]);
    world.model.header_clock(Arc::clone(&world.clock));
    let shared = runner_test::turn::share(&world);
    let handle = tokio::spawn(run_shared(shared, Device::Phone, "build"));
    wait_for_request(&world, 2).await?;
    let task = created_task(&world).ok_or("no created task")?;
    world.append_task_stopped(task).await?;
    let joined = async move { handle.await.map_err(|e| format!("{e:?}")) };
    let outcome = drive(&world.clock, joined).await?;
    let outcome = outcome.map_err(|_e| "join failed".to_owned())?;
    let _outcome = outcome.map_err(|e| format!("{e:?}"))?;
    assert!(!kind_names(&world, &task).contains(&"claim_checked".to_owned()));
    Ok(())
}

#[tokio::test]
async fn t_claims_link_3() -> Result<(), String> {
    let setup = WorldSetup::default();
    let world = TurnWorld::start(&setup).await?;
    world.script(vec![model_replies_text(
        "Docs: https://example.com or mailto:a@b.c or `~/x`",
        false,
    )]);
    let outcome = world
        .send(Device::Phone, "links")
        .await
        .map_err(|e| format!("{e:?}"))?;
    let task = task_of(&outcome);
    assert!(!kind_names(&world, &task).contains(&"claim_checked".to_owned()));
    Ok(())
}

#[tokio::test]
async fn dead_link_without_file_is_marked() -> Result<(), String> {
    let setup = WorldSetup::default();
    let world = TurnWorld::start(&setup).await?;
    world.script(vec![model_replies_text(
        "\u{5728}\u{9019}\u{88e1} [r](file:///Users/x/r.md)",
        false,
    )]);
    let outcome = world
        .send(Device::Phone, "report")
        .await
        .map_err(|e| format!("{e:?}"))?;
    let task = task_of(&outcome);
    let claim_event = task_kinds(&world, &task).into_iter().find_map(|k| {
        let KnownTaskEventKind::ClaimChecked {
            covers_claim,
            verified,
            reasons,
        } = k
        else {
            return None;
        };
        Some((covers_claim, verified, reasons))
    });
    let Some((covers_claim, verified, reasons)) = claim_event else {
        return Err("expected ClaimChecked event".to_owned());
    };
    assert!(covers_claim);
    assert!(!verified);
    assert!(reasons.iter().any(|r| r.code == CODE_CLAIM_CHAT_DEAD_LINK));
    Ok(())
}

#[tokio::test]
async fn claim_check_sends_no_model_request() -> Result<(), String> {
    let setup = WorldSetup {
        tools: all_specs(),
        scripts: vec![("build-step".to_owned(), CommandScript::failing("err", 2))],
        ..WorldSetup::default()
    };
    let world = TurnWorld::start(&setup).await?;
    world.script(vec![
        model_tool_calls(
            vec![call("shell", "{\"line\":\"build-step\"}", "c1")],
            "tool_calls",
            false,
        ),
        model_replies_text("\u{5df2}\u{5b8c}\u{6210}", false),
    ]);
    let outcome = world
        .send(Device::Phone, "build")
        .await
        .map_err(|e| format!("{e:?}"))?;
    let task = task_of(&outcome);
    assert_eq!(chat_requests(&world.model).len(), 2);
    let claim_event = task_kinds(&world, &task).into_iter().find_map(|k| {
        let KnownTaskEventKind::ClaimChecked {
            covers_claim,
            verified,
            reasons,
        } = k
        else {
            return None;
        };
        Some((covers_claim, verified, reasons))
    });
    let Some((covers_claim, verified, reasons)) = claim_event else {
        return Err("expected ClaimChecked event".to_owned());
    };
    assert!(covers_claim);
    assert!(!verified);
    assert!(
        reasons
            .iter()
            .any(|r| r.code == CODE_CLAIM_CHAT_STEP_FAILED)
    );
    Ok(())
}
