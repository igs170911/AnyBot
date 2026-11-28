use anybot_context as _;
use anybot_domain as _;
use anybot_foundation as _;
use anybot_messages as _;
use anybot_models as _;
use anybot_runner as _;
use anybot_store_client as _;
use anybot_tools as _;
use proptest as _;
use serde_json as _;
use sha2 as _;
use test_harness as _;
use thiserror as _;

use anybot_foundation::{BotId, SystemClock, TimestampMs};
use anybot_protocol::{
    AnswerOrigin, EvidenceKind, GateRecord, GateStep, JudgementSource, KnownTaskEventKind,
};
use anybot_runner::TurnOutcome;
use anybot_runner::yolo_release::release_for_yolo;
use anybot_trust::pattern::parse_pattern;
use anybot_trust::{AlwaysEntry, AlwaysList, GateConfig, UserRules};
use runner_test::fake_scripts::CommandScript;
use runner_test::trust_world::{MAC, cards_of, first_turn, mac_setup, modes};
use runner_test::turn::{Device, TurnWorld};
use runner_test::turn_helpers::{task_kinds, task_of};
use runner_test::turn_tool_helpers::{call, one_tool_step};
use test_harness::advance_clock;

const PUSH: &str = "{\"line\":\"git push\"}";

async fn push_world(direct: bool) -> Result<TurnWorld, String> {
    let mut setup = mac_setup(MAC);
    setup
        .scripts
        .push(("git push".to_owned(), CommandScript::ok("pushed")));
    let pattern = parse_pattern("shell(git push*)").map_err(|e| format!("{e:?}"))?;
    setup.gate = GateConfig {
        user: UserRules {
            ask_patterns: vec![pattern],
            always: AlwaysList {
                entries: vec![AlwaysEntry {
                    action: "command git push on Work Mac".to_owned(),
                    added_at: TimestampMs::from_millis(1),
                    from_bot: BotId::generate(&SystemClock),
                }],
            },
            ..UserRules::default()
        },
        ..modes(false, direct)
    };
    TurnWorld::start(&setup).await
}

async fn command(world: &TurnWorld, text: &str) -> Result<TurnOutcome, String> {
    let _advanced = advance_clock(&world.clock, std::time::Duration::from_millis(100));
    world
        .send(Device::Desktop, text)
        .await
        .map_err(|e| format!("{e:?}"))
}

fn executed(world: &TurnWorld, outcome: &TurnOutcome) -> usize {
    task_kinds(world, &task_of(outcome))
        .iter()
        .filter(|k| {
            matches!(k, KnownTaskEventKind::EvidenceAdded { record } if record.kind == EvidenceKind::Executed)
        })
        .count()
}

fn gates(world: &TurnWorld, outcome: &TurnOutcome) -> Vec<GateRecord> {
    task_kinds(world, &task_of(outcome))
        .into_iter()
        .filter_map(|k| {
            if let KnownTaskEventKind::GateResult { gate, .. } = k {
                Some(gate)
            } else {
                None
            }
        })
        .collect()
}

async fn push_again(world: &TurnWorld, id: &str) -> Result<TurnOutcome, String> {
    let _advanced = advance_clock(&world.clock, std::time::Duration::from_millis(100));
    world.script(one_tool_step(vec![call("shell", PUSH, id)]));
    world
        .send(Device::Phone, "push again")
        .await
        .map_err(|e| format!("{e:?}"))
}

#[tokio::test]
async fn t_autonomy_patterns_2() -> Result<(), String> {
    let world = push_world(true).await?;
    let first = first_turn(&world, "shell", PUSH).await?;
    assert_eq!(cards_of(&task_kinds(&world, &task_of(&first))).len(), 1);
    assert_eq!(executed(&world, &first), 0);
    let before = world.chat_requests().len();
    let reply = command(&world, "/always-approve").await?;
    assert_eq!(world.chat_requests().len(), before);
    assert_eq!(executed(&world, &reply), 0);
    let second = push_again(&world, "c2").await?;
    assert_eq!(cards_of(&task_kinds(&world, &task_of(&second))).len(), 0);
    assert_eq!(executed(&world, &second), 1);
    let record = gates(&world, &second).pop().ok_or("missing gate record")?;
    assert_eq!(record.gate, GateStep::Relax);
    assert_eq!(record.source, JudgementSource::AlwaysApprove);
    Ok(())
}

#[tokio::test]
async fn yolo_on_releases_waiting_card() -> Result<(), String> {
    let world = push_world(true).await?;
    let mut first = first_turn(&world, "shell", PUSH).await?;
    command(&world, "/yolo on").await?;
    let waiting = std::mem::take(&mut first.pending);
    let kept = release_for_yolo(world.runner(), waiting)
        .await
        .map_err(|e| e.to_string())?;
    assert!(kept.is_empty());
    assert_eq!(executed(&world, &first), 1);
    let origin = task_kinds(&world, &task_of(&first))
        .into_iter()
        .find_map(|k| {
            if let KnownTaskEventKind::CardAnswered { origin, .. } = k {
                Some(origin)
            } else {
                None
            }
        });
    assert_eq!(origin, Some(AnswerOrigin::Auto));
    Ok(())
}

#[tokio::test]
async fn yolo_off_closes_direct_it_opened() -> Result<(), String> {
    let world = push_world(false).await?;
    command(&world, "/yolo").await?;
    assert!(world.runner_gate_config().modes.direct_mode);
    command(&world, "/yolo off").await?;
    assert!(!world.runner_gate_config().modes.direct_mode);
    assert!(!world.runner_gate_config().modes.yolo);
    Ok(())
}

#[tokio::test]
async fn yolo_off_keeps_direct_it_did_not_open() -> Result<(), String> {
    let world = push_world(true).await?;
    command(&world, "/yolo").await?;
    command(&world, "/yolo off").await?;
    assert!(world.runner_gate_config().modes.direct_mode);
    assert!(!world.runner_gate_config().modes.yolo);
    Ok(())
}

#[tokio::test]
async fn status_reports_without_changing() -> Result<(), String> {
    let world = push_world(false).await?;
    let reply = command(&world, "/yolo maybe").await?;
    assert!(!world.runner_gate_config().modes.yolo);
    assert_eq!(executed(&world, &reply), 0);
    Ok(())
}
