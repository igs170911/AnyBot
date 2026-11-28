use anybot_context as _;
use anybot_domain as _;
use anybot_foundation as _;
use anybot_models as _;
use anybot_runner as _;
use anybot_store_client as _;
use anybot_tools as _;
use proptest as _;
use serde_json as _;
use sha2 as _;
use test_harness as _;
use thiserror as _;

use anybot_messages::{Lang, Params};
use anybot_protocol::{
    AnswerOrigin, CardAnswer, EvidenceKind, GateStep, JudgementSource, KnownTaskEventKind,
};
use anybot_runner::TurnOutcome;
use runner_test::card_turn::{AskedTurn, CardOrEnd, script_until_settled};
use runner_test::trust_world::{MAC, cards_of, first_card, mac_setup, modes};
use runner_test::turn::TurnWorld;
use runner_test::turn_helpers::{fmt_text, task_kinds, task_of};
use runner_test::turn_tool_helpers::{call, one_tool_step};
use std::sync::Arc;
use std::time::Duration;
use test_harness::{advance_clock, sleep_real};

const MAKE: &str = "{\"command\":\"make test\"}";
const REDIRECT: &str = "{\"command\":\"ls > out.txt\"}";

fn executed(world: &TurnWorld, outcome: &TurnOutcome) -> usize {
    task_kinds(world, &task_of(outcome))
        .iter()
        .filter(|k| {
            matches!(k, KnownTaskEventKind::EvidenceAdded { record } if record.kind == EvidenceKind::Executed)
        })
        .count()
}

fn executed_now(world: &TurnWorld, task: &anybot_foundation::TaskId) -> usize {
    task_kinds(world, task)
        .iter()
        .filter(|k| {
            matches!(k, KnownTaskEventKind::EvidenceAdded { record } if record.kind == EvidenceKind::Executed)
        })
        .count()
}

async fn waiting_shell(command: &str, seconds: i64) -> Result<(Arc<TurnWorld>, AskedTurn), String> {
    let world = runner_test::turn::arc_world(TurnWorld::start(&mac_setup(MAC)).await?);
    world.trust.set_smart_seconds(seconds);
    let asked = first_card(&world, "shell", command).await?;
    assert_eq!(cards_of(&task_kinds(&world, &asked.task)).len(), 1);
    Ok((world, asked))
}

async fn advance(world: &TurnWorld, seconds: u64) {
    let _wakes = advance_clock(&world.clock, Duration::from_secs(seconds));
    sleep_real(Duration::from_millis(20)).await;
}

#[tokio::test]
async fn smart_runs_a_default_card_after_the_wait() -> Result<(), String> {
    let (world, asked) = waiting_shell(MAKE, 300).await?;
    advance(&world, 301).await;
    let outcome = asked.finish().await?;
    assert!(outcome.pending.is_empty());
    assert_eq!(executed(&world, &outcome), 1);
    let kinds = task_kinds(&world, &task_of(&outcome));
    let note = fmt_text(
        Lang::ZhTw,
        "gate-smart-auto-allowed",
        &Params::new().with_text("seconds", "300"),
    )?;
    assert!(kinds.iter().any(|k| matches!(
        k,
        KnownTaskEventKind::ToolCompleted { summary, .. } if summary.contains(&note)
    )));
    let source = kinds.iter().find_map(|k| {
        if let KnownTaskEventKind::GateResult { gate, .. } = k
            && gate.gate == GateStep::AskYou
        {
            Some(gate.source)
        } else {
            None
        }
    });
    assert_eq!(source, Some(JudgementSource::RuleTimeout));
    Ok(())
}

#[tokio::test]
async fn smart_waits_before_deadline() -> Result<(), String> {
    let (world, asked) = waiting_shell(MAKE, 300).await?;
    advance(&world, 299).await;
    assert!(asked.is_running());
    assert_eq!(executed_now(&world, &asked.task), 0);
    asked.answer(&world, CardAnswer::Deny).await?;
    Ok(())
}

#[tokio::test]
async fn smart_never_answers_every_time_card() -> Result<(), String> {
    let (world, asked) = waiting_shell(REDIRECT, 30).await?;
    advance(&world, 600).await;
    assert!(asked.is_running());
    assert_eq!(executed_now(&world, &asked.task), 0);
    asked.answer(&world, CardAnswer::Deny).await?;
    Ok(())
}

#[tokio::test]
async fn smart_off_when_admin_locked() -> Result<(), String> {
    let mut setup = mac_setup(MAC);
    setup.gate = anybot_trust::GateConfig {
        modes: anybot_trust::ModeFlags {
            mdm_locks_direct: true,
            ..modes(false, false).modes
        },
        ..anybot_trust::GateConfig::plain()
    };
    let world = runner_test::turn::arc_world(TurnWorld::start(&setup).await?);
    world.trust.set_smart_seconds(30);
    let settled =
        script_until_settled(&world, one_tool_step(vec![call("shell", MAKE, "c1")])).await?;
    advance(&world, 600).await;
    assert_locked_idle(&world, settled).await
}

async fn assert_locked_idle(world: &Arc<TurnWorld>, settled: CardOrEnd) -> Result<(), String> {
    match settled {
        CardOrEnd::Ended(outcome) => {
            assert_eq!(executed(world, &outcome), 0);
            Ok(())
        }
        CardOrEnd::Asked(asked) => {
            assert!(
                asked.is_running(),
                "a locked machine must not answer by itself"
            );
            assert_eq!(executed_now(world, &asked.task), 0);
            asked.answer(world, CardAnswer::Deny).await?;
            Ok(())
        }
    }
}

#[tokio::test]
async fn expiry_time_follows_the_oldest_answerable_card() -> Result<(), String> {
    let (world, asked) = waiting_shell(MAKE, 300).await?;
    advance(&world, 299).await;
    assert!(asked.is_running());
    advance(&world, 2).await;
    let outcome = asked.finish().await?;
    assert_eq!(executed(&world, &outcome), 1);
    Ok(())
}

#[tokio::test]
async fn smart_answer_is_recorded_as_automatic() -> Result<(), String> {
    let (world, asked) = waiting_shell(MAKE, 300).await?;
    advance(&world, 301).await;
    let outcome = asked.finish().await?;
    let origin = task_kinds(&world, &task_of(&outcome))
        .into_iter()
        .find_map(|k| {
            if let KnownTaskEventKind::CardAnswered { origin, answer } = k {
                Some((origin, answer))
            } else {
                None
            }
        });
    assert_eq!(origin, Some((AnswerOrigin::Auto, CardAnswer::AllowOnce)));
    Ok(())
}
