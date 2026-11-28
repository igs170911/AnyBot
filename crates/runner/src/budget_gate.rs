use std::collections::BTreeSet;

use anybot_domain::TaskView;
use anybot_domain::task_view::BudgetOver;
use anybot_models::UsageReport;
use anybot_protocol::{BudgetItem, TaskEvent};

use crate::error::TurnError;
use crate::runner::{TurnCtx, TurnRunner};

pub const TURN_STEP_CAP: u64 = 50_000;

pub(crate) async fn check_step_budget(
    runner: &TurnRunner,
    ctx: &TurnCtx,
) -> Result<Option<BudgetOver>, TurnError> {
    let events = runner
        .record
        .task_events(ctx.task)
        .await
        .map_err(TurnError::Record)?;
    Ok(step_over(&events, runner.clock.now_ms().millis()))
}

#[must_use]
pub(crate) fn step_over(events: &[TaskEvent], now_ms: u64) -> Option<BudgetOver> {
    let mut view = replay_at(events, now_ms);
    if view.budget.over.is_none() && view.budget.turn_steps >= TURN_STEP_CAP {
        return Some(BudgetOver {
            item: BudgetItem::Steps,
            used: TURN_STEP_CAP,
            limit: TURN_STEP_CAP,
        });
    }
    anybot_domain::task_budget::note_ask_attempt(&mut view);
    view.budget.over
}

pub(crate) async fn check_action_budget(
    runner: &TurnRunner,
    ctx: &TurnCtx,
) -> Result<Option<BudgetOver>, TurnError> {
    let events = runner
        .record
        .task_events(ctx.task)
        .await
        .map_err(TurnError::Record)?;
    Ok(action_over(&events, runner.clock.now_ms().millis()))
}

#[must_use]
pub(crate) fn action_over(events: &[TaskEvent], now_ms: u64) -> Option<BudgetOver> {
    let mut view = replay_at(events, now_ms);
    anybot_domain::task_budget::note_action_attempt(&mut view);
    view.budget.over
}

pub(crate) async fn current_over(
    runner: &TurnRunner,
    ctx: &TurnCtx,
) -> Result<Option<BudgetOver>, TurnError> {
    let events = runner
        .record
        .task_events(ctx.task)
        .await
        .map_err(TurnError::Record)?;
    let view = replay_at(&events, runner.clock.now_ms().millis());
    Ok(view.budget.over)
}

#[must_use]
pub(crate) fn token_delta(usage: &UsageReport) -> Option<u64> {
    let UsageReport::Present(values) = usage else {
        return None;
    };
    let total = values
        .input_total()
        .saturating_add(values.output_tokens)
        .saturating_add(values.thinking_tokens);
    if total == 0 {
        return None;
    }
    Some(total)
}

fn replay_at(events: &[TaskEvent], now_ms: u64) -> TaskView {
    let mut view = anybot_domain::initial_view();
    let mut seen = BTreeSet::new();
    anybot_domain::task_replay::fold(&mut view, events, &mut seen);
    if view.budget.turn_start_ms.is_some() {
        view.budget.last_ms = view.budget.last_ms.max(now_ms);
    }
    view
}

#[cfg(test)]
mod tests {
    use super::{TURN_STEP_CAP, action_over, step_over};
    use anybot_domain::task_view::{BudgetOver, UNLIMITED};
    use anybot_foundation::{
        BotId, CallId, ContentDigest, DeviceId, EventId, EvidenceId, SystemClock, TaskId,
        TimestampMs, TurnId,
    };
    use anybot_protocol::{
        ActionOutcome, BudgetItem, CheckSource, ComputerPlatform, ComputerRef, EvidenceKind,
        EvidencePayload, EvidenceRecord, EvidenceShortId, GateNo, GateOutcome, GateRecord,
        JudgementSource, KnownTaskEventKind, ModelName, RuleLayer, TaskEvent, TaskEventKind,
        TaskOpener, ThinkingChoice, TurnInitiator,
    };

    fn ev(seq: u64, kind: KnownTaskEventKind) -> TaskEvent {
        TaskEvent {
            event_id: EventId::generate(&SystemClock),
            seq,
            at_ms: TimestampMs::from_millis(seq.saturating_mul(10)),
            kind: TaskEventKind::Known(kind),
        }
    }

    fn created() -> KnownTaskEventKind {
        KnownTaskEventKind::TaskCreated {
            goal: "g".to_owned(),
            opened_by: TaskOpener::You,
            bot: BotId::generate(&SystemClock),
            parent: None,
        }
    }

    fn turn_started() -> KnownTaskEventKind {
        KnownTaskEventKind::TurnStarted {
            turn: 1,
            initiator: TurnInitiator::You {
                device: DeviceId::generate(&SystemClock),
            },
            computer: ComputerRef {
                name: "c".to_owned(),
                platform: ComputerPlatform::BotMachine,
            },
            model: ModelName {
                provider: "OpenAi".to_owned(),
                base_url: "http://127.0.0.1:9/v1".to_owned(),
                model: "m".to_owned(),
            },
            thinking: ThinkingChoice::Auto,
        }
    }

    #[test]
    fn step_cap_blocks_at_50_000() {
        let mut events = vec![
            ev(1, created()),
            ev(
                2,
                KnownTaskEventKind::BudgetLimit {
                    item: BudgetItem::Steps,
                    limit: UNLIMITED,
                },
            ),
            ev(3, turn_started()),
        ];
        for i in 0..49_999u64 {
            events.push(ev(
                4u64.saturating_add(i),
                KnownTaskEventKind::BudgetUsage {
                    item: BudgetItem::Steps,
                    delta: 1,
                },
            ));
        }
        assert_eq!(step_over(&events, 1_000_000), None);
        events.push(ev(
            50_003,
            KnownTaskEventKind::BudgetUsage {
                item: BudgetItem::Steps,
                delta: 1,
            },
        ));
        assert_eq!(
            step_over(&events, 1_000_000),
            Some(BudgetOver {
                item: BudgetItem::Steps,
                used: TURN_STEP_CAP,
                limit: TURN_STEP_CAP,
            })
        );
    }

    fn verification_gate() -> GateRecord {
        GateRecord::new(
            GateNo(1),
            RuleLayer::User,
            "allow-read".to_owned(),
            JudgementSource::Rule,
            GateOutcome::Allow,
            true,
        )
    }

    fn verification_evidence() -> EvidenceRecord {
        EvidenceRecord {
            id: EvidenceId::generate(&SystemClock),
            short: EvidenceShortId(1),
            at: TimestampMs::from_millis(60),
            task: TaskId::generate(&SystemClock),
            turn: TurnId::generate(&SystemClock),
            computer: "c".to_owned(),
            workdir: "/tmp".to_owned(),
            tool: "read_file".to_owned(),
            classes: Vec::new(),
            gate: verification_gate(),
            kind: EvidenceKind::Seen,
            outcome: ActionOutcome::Completed,
            payload: EvidencePayload::Seen {
                path: "out.csv".to_owned(),
                digest: ContentDigest::sha256(b"ok"),
                size: 2,
            },
            verification: None,
            source: JudgementSource::ProgramCheck,
            ungated: false,
        }
    }

    #[test]
    fn verification_tools_do_not_count_as_tool_calls() {
        let call_id = CallId::generate(&SystemClock);
        let events = vec![
            ev(1, created()),
            ev(
                2,
                KnownTaskEventKind::BudgetLimit {
                    item: BudgetItem::ToolCalls,
                    limit: 1,
                },
            ),
            ev(3, turn_started()),
            ev(4, KnownTaskEventKind::VerificationStarted),
            ev(
                5,
                KnownTaskEventKind::GateResult {
                    call_id,
                    gate: verification_gate(),
                },
            ),
            ev(
                6,
                KnownTaskEventKind::EvidenceAdded {
                    record: Box::new(verification_evidence()),
                },
            ),
            ev(
                7,
                KnownTaskEventKind::CriterionPassed {
                    index: 0,
                    source: CheckSource::Program,
                },
            ),
        ];
        assert_eq!(action_over(&events, 100), None);
    }
}
