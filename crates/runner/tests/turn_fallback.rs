use anybot_domain as _;
use anybot_messages as _;
use anybot_runner as _;
use anybot_store_client as _;
use anybot_tools as _;
use anybot_trust as _;
use proptest as _;
use sha2 as _;
use thiserror as _;

use anybot_context::Manifest;
use anybot_models::{
    Calibration, EstimateInput, apply_calibration, fits, formula_tokens, usable_context,
};
use anybot_protocol::{KnownTaskEventKind, TaskDetail, TaskStatus, TurnEndReason};
use runner_test::turn::{Device, FallbackTarget, TurnWorld, WorldSetup};
use runner_test::turn_helpers::{
    chat_requests, drive, kind_names, raw_bytes_of, state_of, task_kinds, task_of,
};
use test_harness::{
    ModelFormat, model_calls_tool, model_replies_text, model_reply_status, model_start_with_format,
};

#[tokio::test]
async fn t_turn_loop_4() -> Result<(), String> {
    let (m2, m2_port) = model_start_with_format(ModelFormat::OpenAi).await?;
    let files = build_thirty_files();
    let setup = WorldSetup {
        bot_context: 128_000,
        reserved_output: 1_024,
        primary_name: Some("m1".to_owned()),
        fallbacks: vec![FallbackTarget {
            format: ModelFormat::OpenAi,
            port: m2_port,
            model: "m2".to_owned(),
            bot_context: 16_000,
        }],
        files,
        ..WorldSetup::default()
    };
    let world = TurnWorld::start(&setup).await?;
    world.script(round_one_steps());
    let first = world
        .send(Device::Phone, "read 30 files")
        .await
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!(first.reason, TurnEndReason::Answered);
    let task = task_of(&first);
    let r1_actions = world.gated.logged().len();
    assert_eq!(r1_actions, 30);

    world.script(vec![
        model_reply_status(503, "overloaded"),
        model_reply_status(503, "overloaded"),
        model_reply_status(503, "overloaded"),
    ]);
    m2.script_set(vec![model_replies_text("ok", false)]);
    let second = Box::pin(drive(
        &world.clock,
        world.continue_from(task, Device::Phone, Some("go on")),
    ))
    .await?
    .map_err(|e| format!("{e:?}"))?;
    assert_eq!(second.reason, TurnEndReason::Answered);
    assert_eq!(world.gated.logged().len(), r1_actions);
    assert_eq!(chat_requests(&m2).len(), 1);
    assert_swap_and_m2_budget(&world, &m2, task)?;
    Ok(())
}

fn build_thirty_files() -> Vec<(String, String)> {
    let body = "word ".repeat(400);
    (0..30)
        .map(|i| (format!("doc_{i}.txt"), body.clone()))
        .collect()
}

fn round_one_steps() -> Vec<test_harness::Step> {
    let mut steps: Vec<test_harness::Step> = (0..30)
        .map(|i| {
            model_calls_tool(
                "read_file",
                serde_json::json!({ "path": format!("doc_{i}.txt") }),
                false,
            )
        })
        .collect();
    steps.push(model_replies_text("round 1 done", false));
    steps
}

fn assert_swap_and_m2_budget(
    world: &TurnWorld,
    m2: &test_harness::FakeModel,
    task: anybot_foundation::TaskId,
) -> Result<(), String> {
    let kinds = task_kinds(world, &task);
    let swap_pos = kinds
        .iter()
        .position(|k| matches!(k, KnownTaskEventKind::ModelSwapped { from, to, .. } if from == "m1" && to == "m2"))
        .ok_or("missing ModelSwapped m1 -> m2")?;
    let mut m2_manifest_id: Option<String> = None;
    for kind in kinds.iter().skip(swap_pos) {
        if let KnownTaskEventKind::ContextBuilt {
            manifest_id,
            model,
            estimate_tokens,
        } = kind
        {
            assert_eq!(model.model, "m2");
            assert!(*estimate_tokens <= usable_context(16_000, Some(16_000), None, 1_024));
            m2_manifest_id = Some(manifest_id.clone());
            break;
        }
        if matches!(kind, KnownTaskEventKind::ModelAnswer { .. }) {
            return Err("ContextBuilt must appear before M2 answer".to_owned());
        }
    }
    let manifest_id = m2_manifest_id.ok_or("missing ContextBuilt for M2 after ModelSwapped")?;
    let bytes = raw_bytes_of(&world.raw, &manifest_id).ok_or("missing M2 manifest in raw")?;
    let manifest: Manifest = serde_json::from_slice(&bytes).map_err(|e| format!("{e}"))?;
    let m2_usable = usable_context(16_000, Some(16_000), None, 1_024);
    assert_eq!(manifest.model.model, "m2");
    assert_eq!(manifest.model.context_tokens, 16_000);
    assert_eq!(manifest.budget.usable, m2_usable);
    assert!(manifest.budget.estimate_calibrated <= m2_usable);

    let recorded = chat_requests(m2);
    let req_body = recorded.first().ok_or("missing M2 request")?;
    let value: serde_json::Value =
        serde_json::from_str(&req_body.body).map_err(|e| format!("{e}"))?;
    let (all_text, msg_count) = collect_messages_text(&value);
    let raw_tokens = formula_tokens(&EstimateInput {
        text: &all_text,
        messages: msg_count,
        images: 0,
    });
    let with_margin = apply_calibration(raw_tokens, &Calibration::new());
    assert!(
        fits(with_margin, m2_usable),
        "M2 estimate+margin {with_margin} exceeds usable {m2_usable}"
    );
    Ok(())
}

fn collect_messages_text(value: &serde_json::Value) -> (String, u64) {
    let mut text = String::new();
    let Some(list) = value.get("messages").and_then(|v| v.as_array()) else {
        return (text, 0);
    };
    for msg in list {
        if let Some(content) = msg.get("content").and_then(|c| c.as_str()) {
            text.push_str(content);
        }
        if let Some(calls) = msg.get("tool_calls") {
            text.push_str(&calls.to_string());
        }
    }
    (text, u64::try_from(list.len()).unwrap_or(0))
}

#[tokio::test]
async fn fallback_also_fails_marks_failed() -> Result<(), String> {
    let (m2, m2_port) = model_start_with_format(ModelFormat::OpenAi).await?;
    let setup = WorldSetup {
        bot_context: 32_000,
        reserved_output: 1_024,
        primary_name: Some("m1".to_owned()),
        fallbacks: vec![FallbackTarget {
            format: ModelFormat::OpenAi,
            port: m2_port,
            model: "m2".to_owned(),
            bot_context: 16_000,
        }],
        ..WorldSetup::default()
    };
    let world = TurnWorld::start(&setup).await?;
    world.script(vec![
        model_reply_status(503, "busy"),
        model_reply_status(503, "busy"),
        model_reply_status(503, "busy"),
    ]);
    m2.script_set(vec![model_reply_status(401, "invalid key")]);
    let outcome = Box::pin(drive(&world.clock, world.send(Device::Phone, "hi")))
        .await?
        .map_err(|e| format!("{e:?}"))?;
    let task = task_of(&outcome);
    assert_eq!(chat_requests(&m2).len(), 1);
    assert_eq!(
        outcome.reason,
        TurnEndReason::ModelFailed {
            code: "model-auth".to_owned()
        }
    );
    let names = kind_names(&world, &task);
    assert!(names.iter().any(|n| n == "model_swapped"));
    assert_eq!(
        state_of(&world, &task)?,
        (
            TaskStatus::Failed,
            Some(TaskDetail::Error {
                reason: "model-auth".to_owned()
            })
        )
    );
    Ok(())
}

#[tokio::test]
async fn unreachable_model_marks_stuck() -> Result<(), String> {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").map_err(|e| format!("{e}"))?;
    let dead_port = listener.local_addr().map_err(|e| format!("{e}"))?.port();
    drop(listener);
    let setup = WorldSetup {
        primary_port_override: Some(dead_port),
        ..WorldSetup::default()
    };
    let world = TurnWorld::start(&setup).await?;
    let outcome = Box::pin(drive(&world.clock, world.send(Device::Phone, "hi")))
        .await?
        .map_err(|e| format!("{e:?}"))?;
    let task = task_of(&outcome);
    assert_eq!(outcome.reason, TurnEndReason::ModelUnreachable);
    let names = kind_names(&world, &task);
    assert!(names.iter().any(|n| n == "model_unreachable"));
    assert_eq!(
        state_of(&world, &task)?,
        (TaskStatus::Stuck, Some(TaskDetail::ModelUnreachable))
    );
    Ok(())
}
