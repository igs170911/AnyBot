use anybot_context as _;
use anybot_domain as _;
use anybot_foundation as _;
use anybot_messages as _;
use anybot_models as _;
use anybot_runner as _;
use anybot_tools as _;
use anybot_trust as _;
use proptest as _;
use serde_json as _;
use sha2 as _;
use thiserror as _;

use anybot_protocol::{KnownTaskEventKind, ReportOutcome, ReportSummary, TaskStatus};
use runner_test::turn::{Device, TurnWorld, WorldSetup, arc_world};
use runner_test::turn_helpers::{chat_requests, delivery_of, task_kinds, task_of};
use runner_test::turn_interrupt::edit_then_interrupt;
use runner_test::turn_tool_helpers::{all_specs, edit_call};
use test_harness::{Step, model_cut_off, model_replies_text, model_tool_calls};

fn edit_args(path: &str, text: &str) -> String {
    format!("{{\"path\":\"{path}\",\"old_text\":\"\",\"new_text\":\"{text}\"}}")
}

fn edit_then_cut_off() -> Vec<Step> {
    vec![
        model_tool_calls(
            vec![edit_call(&edit_args("a.txt", "1"), "c1")],
            "tool_calls",
            false,
        ),
        model_cut_off("half an ans", true),
        model_cut_off("half an ans", true),
        model_cut_off("half an ans", true),
        model_cut_off("half an ans", true),
    ]
}

#[tokio::test]
async fn t_report_delivery_8() -> Result<(), String> {
    let setup = WorldSetup {
        tools: all_specs(),
        ..WorldSetup::default()
    };
    let world = TurnWorld::start(&setup).await?;
    world.script(edit_then_cut_off());
    let outcome = Box::pin(runner_test::turn_helpers::drive(
        &world.clock,
        world.send(Device::Phone, "write a"),
    ))
    .await?;
    let task = task_of(&outcome.map_err(|e| format!("{e:?}"))?);
    let report = delivery_of(&world, task)?;
    assert_eq!(
        report.summary,
        ReportSummary::Missing {
            code: anybot_trust::CODE_REPORT_SUMMARY_MISSING.to_owned()
        }
    );
    assert_eq!(report.level_counts.verified, 0);
    assert_eq!(report.steps.len(), 1);
    assert!(
        report
            .steps
            .iter()
            .all(|s| s.outcome == ReportOutcome::Completed && !s.evidence_ids.is_empty())
    );
    let asked = task_kinds(&world, &task)
        .into_iter()
        .filter(|k| matches!(k, KnownTaskEventKind::ModelAsked { .. }))
        .count();
    assert_eq!(chat_requests(&world.model).len(), asked);
    Ok(())
}

fn has_report(world: &TurnWorld, task: anybot_foundation::TaskId) -> bool {
    task_kinds(world, &task)
        .iter()
        .any(|k| matches!(k, KnownTaskEventKind::DeliveryReport { .. }))
}

fn has_step_and_evidence(world: &TurnWorld, task: anybot_foundation::TaskId) -> bool {
    let kinds = task_kinds(world, &task);
    let step = kinds
        .iter()
        .any(|k| matches!(k, KnownTaskEventKind::ToolCompleted { .. }));
    let evidence = kinds
        .iter()
        .any(|k| matches!(k, KnownTaskEventKind::EvidenceAdded { .. }));
    step && evidence
}

async fn started_world() -> Result<std::sync::Arc<TurnWorld>, String> {
    let setup = WorldSetup {
        tools: all_specs(),
        ..WorldSetup::default()
    };
    Ok(arc_world(TurnWorld::start(&setup).await?))
}

#[tokio::test]
async fn t_report_delivery_12() -> Result<(), String> {
    let chat = started_world().await?;
    chat.script(vec![model_replies_text("hello", false)]);
    let outcome = chat.send(Device::Phone, "hi").await;
    let chat_task = task_of(&outcome.map_err(|e| format!("{e:?}"))?);
    assert!(!has_report(&chat, chat_task));

    let stopped = started_world().await?;
    let stop = KnownTaskEventKind::TaskStopped;
    let stopped_task = edit_then_interrupt(&stopped, &edit_args("a.txt", "1"), stop).await?;
    assert!(!has_report(&stopped, stopped_task));
    assert!(has_step_and_evidence(&stopped, stopped_task));
    assert_eq!(stopped.task_state(stopped_task)?.0, TaskStatus::Stopped);

    let cancelled = started_world().await?;
    let cancel = KnownTaskEventKind::TaskCancelled {
        reason: "user".to_owned(),
    };
    let cancelled_task = edit_then_interrupt(&cancelled, &edit_args("a.txt", "1"), cancel).await?;
    assert!(!has_report(&cancelled, cancelled_task));
    assert!(has_step_and_evidence(&cancelled, cancelled_task));
    Ok(())
}
