use anybot_foundation::TurnId;
use anybot_models::{Classified, FailKind, GatewayFail, StopKind};
use anybot_protocol::{
    ActionOutcome, EvidenceKind, EvidencePayload, KnownTaskEventKind, TaskEvent, TaskEventKind,
    TurnEndReason,
};
use anybot_trust::{StepFact, TurnFacts, check_reply, is_send_tool};

use crate::error::TurnError;
use crate::runner::{TurnCtx, TurnRunner};

pub const CODE_MODEL_FAILED: &str = "turn-model-failed";

#[must_use]
pub fn end_reason(fail: &GatewayFail) -> TurnEndReason {
    match fail {
        GatewayFail::Classified { classified, .. } | GatewayFail::Exhausted { classified, .. } => {
            classified_reason(classified)
        }
        GatewayFail::MissingKey
        | GatewayFail::KeyDestination
        | GatewayFail::Status(_)
        | GatewayFail::Transport(_)
        | GatewayFail::Stopped => TurnEndReason::ModelFailed {
            code: CODE_MODEL_FAILED.to_owned(),
        },
    }
}

fn classified_reason(classified: &Classified) -> TurnEndReason {
    if classified.kind == FailKind::Budget {
        return TurnEndReason::ContextBudget {
            deficit_tokens: None,
        };
    }
    TurnEndReason::ModelFailed {
        code: classified.code.to_owned(),
    }
}

#[must_use]
pub fn end_event(reason: TurnEndReason, reply_delivered: bool) -> KnownTaskEventKind {
    KnownTaskEventKind::TurnEnded {
        reason,
        reply_delivered,
    }
}

pub(crate) async fn maybe_record_claim_check(
    runner: &TurnRunner,
    ctx: &TurnCtx,
    reply_text: &str,
    stop: StopKind,
) -> Result<(), TurnError> {
    if stop == StopKind::Interrupted || reply_text.is_empty() {
        return Ok(());
    }
    let rows = runner
        .record
        .task_events(ctx.task)
        .await
        .map_err(TurnError::Record)?;
    if turn_interrupted(&rows) {
        return Ok(());
    }
    let facts = collect_turn_facts(&rows, ctx.turn_id);
    let verdict = check_reply(reply_text, &facts);
    if verdict.verified {
        return Ok(());
    }
    runner
        .append_task_event(
            &ctx.task,
            KnownTaskEventKind::ClaimChecked {
                covers_claim: verdict.covers_claim,
                verified: verdict.verified,
                reasons: verdict.reasons,
            },
        )
        .await
        .map(|_| ())
}

fn turn_tail(rows: &[TaskEvent]) -> &[TaskEvent] {
    if let Some(idx) = rows.iter().rposition(|row| {
        matches!(
            row.kind,
            TaskEventKind::Known(KnownTaskEventKind::TurnStarted { .. })
        )
    }) && let Some(tail) = rows.get(idx..)
    {
        return tail;
    }
    rows
}

fn turn_interrupted(rows: &[TaskEvent]) -> bool {
    turn_tail(rows).iter().any(|row| {
        matches!(
            row.kind,
            TaskEventKind::Known(
                KnownTaskEventKind::TaskStopped | KnownTaskEventKind::TaskCancelled { .. }
            )
        )
    })
}

fn collect_turn_facts(rows: &[TaskEvent], turn_id: TurnId) -> TurnFacts {
    let mut facts = TurnFacts::default();
    let mut gated_calls: Vec<anybot_foundation::CallId> = Vec::new();
    for row in turn_tail(rows) {
        let TaskEventKind::Known(kind) = &row.kind else {
            continue;
        };
        collect_kind_fact(kind, turn_id, &mut facts, &mut gated_calls);
    }
    facts
}

fn collect_kind_fact(
    kind: &KnownTaskEventKind,
    turn_id: TurnId,
    facts: &mut TurnFacts,
    gated_calls: &mut Vec<anybot_foundation::CallId>,
) {
    if let KnownTaskEventKind::GateResult { call_id, .. } = kind {
        gated_calls.push(*call_id);
        return;
    }
    if let KnownTaskEventKind::ToolFailed { call_id, name, .. } = kind
        && !gated_calls.contains(call_id)
    {
        facts.steps.push(StepFact {
            tool: name.clone(),
            read_only: false,
            outcome: ActionOutcome::Failed,
            exit_code: None,
            output_empty: false,
        });
        return;
    }
    if let KnownTaskEventKind::EvidenceAdded { record } = kind
        && record.turn == turn_id
        && record.kind != EvidenceKind::Approved
    {
        let (exit_code, output_empty) = payload_exec_meta(&record.payload);
        if record.outcome == ActionOutcome::Completed && is_send_tool(&record.tool) {
            facts.sends = facts.sends.saturating_add(1);
        }
        facts.steps.push(StepFact {
            tool: record.tool.clone(),
            read_only: record.gate.read_only,
            outcome: record.outcome,
            exit_code,
            output_empty,
        });
    }
}

fn payload_exec_meta(payload: &EvidencePayload) -> (Option<i32>, bool) {
    if let EvidencePayload::Executed {
        exit_code,
        output_head,
        output_tail,
        output_omitted,
        ..
    } = payload
    {
        return (
            *exit_code,
            output_head.is_empty() && output_tail.is_empty() && *output_omitted == 0,
        );
    }
    (None, false)
}

#[cfg(test)]
#[path = "ending_tests.rs"]
mod ending_tests;
