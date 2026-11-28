use anybot_context as _;
use anybot_domain as _;
use anybot_foundation as _;
use anybot_models as _;
use anybot_runner as _;
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
use anybot_runner::card_timers::{answer_expired_cards, next_expiry_ms};
use runner_test::trust_world::{MAC, cards_of, first_turn, mac_setup, modes};
use runner_test::turn::TurnWorld;
use runner_test::turn_helpers::{fmt_text, task_kinds, task_of};
use std::time::Duration;
use test_harness::advance_clock;

const MAKE: &str = "{\"line\":\"make test\"}";
const REDIRECT: &str = "{\"line\":\"ls > out.txt\"}";

fn executed(world: &TurnWorld, outcome: &TurnOutcome) -> usize {
    task_kinds(world, &task_of(outcome))
        .iter()
        .filter(|k| {
            matches!(k, KnownTaskEventKind::EvidenceAdded { record } if record.kind == EvidenceKind::Executed)
        })
        .count()
}

async fn wait_and_expire(
    world: &TurnWorld,
    outcome: &mut TurnOutcome,
    seconds: u64,
) -> Result<(), String> {
    let _advanced = advance_clock(&world.clock, Duration::from_secs(seconds));
    let waiting = std::mem::take(&mut outcome.pending);
    outcome.pending = answer_expired_cards(world.runner(), waiting)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

async fn parked_read(seconds: i64) -> Result<(TurnWorld, TurnOutcome), String> {
    let world = TurnWorld::start(&mac_setup(MAC)).await?;
    world.trust.set_smart_seconds(seconds);
    let first = first_turn(&world, "shell", MAKE).await?;
    assert_eq!(cards_of(&task_kinds(&world, &task_of(&first))).len(), 1);
    Ok((world, first))
}

#[tokio::test]
async fn smart_runs_a_default_card_after_the_wait() -> Result<(), String> {
    let (world, mut outcome) = parked_read(300).await?;
    wait_and_expire(&world, &mut outcome, 301).await?;
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
    let (world, mut outcome) = parked_read(300).await?;
    wait_and_expire(&world, &mut outcome, 299).await?;
    assert_eq!(outcome.pending.len(), 1);
    assert_eq!(executed(&world, &outcome), 0);
    Ok(())
}

#[tokio::test]
async fn smart_never_answers_every_time_card() -> Result<(), String> {
    let world = TurnWorld::start(&mac_setup(MAC)).await?;
    world.trust.set_smart_seconds(30);
    let mut first = first_turn(&world, "shell", REDIRECT).await?;
    assert_eq!(first.pending.len(), 1);
    wait_and_expire(&world, &mut first, 600).await?;
    assert_eq!(first.pending.len(), 1);
    assert_eq!(executed(&world, &first), 0);
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
    let world = TurnWorld::start(&setup).await?;
    world.trust.set_smart_seconds(30);
    let mut first = first_turn(&world, "shell", MAKE).await?;
    wait_and_expire(&world, &mut first, 600).await?;
    assert_eq!(executed(&world, &first), 0);
    assert!(
        first.pending.len() <= 1,
        "a locked machine must not answer by itself"
    );
    Ok(())
}

#[tokio::test]
async fn expiry_time_follows_the_oldest_answerable_card() -> Result<(), String> {
    let (world, outcome) = parked_read(300).await?;
    let due = next_expiry_ms(world.runner(), &outcome.pending).map_err(|e| e.to_string())?;
    let now = world.clock.now_ms();
    assert_eq!(due, Some(now.saturating_add(300_000)));
    Ok(())
}

#[tokio::test]
async fn smart_answer_is_recorded_as_automatic() -> Result<(), String> {
    let (world, mut outcome) = parked_read(300).await?;
    wait_and_expire(&world, &mut outcome, 301).await?;
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
