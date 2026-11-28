use std::collections::BTreeMap;

use anybot_foundation::CallId;
use anybot_protocol::{
    CheckSource, DeliveryReportRecord, KnownTaskEventKind, ReportAttention, ReportOutcome,
    ReportStep, ReportSummary, TaskEvent, TaskEventKind,
};

use crate::message_codes::CODE_REPORT_SUMMARY_MISSING;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("report input has a replay gap: {gap:?}")]
pub struct ReportFail {
    pub gap: anybot_domain::ReplayGap,
}

struct PendingStep {
    name: String,
    args_summary: String,
    outcome: ReportOutcome,
    evidence_ids: Vec<anybot_foundation::EvidenceId>,
    verify_source: Option<CheckSource>,
}

#[must_use]
pub fn report_summary(reply: Option<&str>) -> ReportSummary {
    match reply {
        Some(text) if !text.is_empty() => ReportSummary::Text(text.to_owned()),
        _ => ReportSummary::Missing {
            code: CODE_REPORT_SUMMARY_MISSING.to_owned(),
        },
    }
}

pub fn delivery_report(
    events: &[TaskEvent],
    goal: &str,
    reply: Option<&str>,
) -> Result<DeliveryReportRecord, ReportFail> {
    let view = anybot_domain::replay(events).map_err(|gap| ReportFail { gap })?;
    let mut steps: BTreeMap<String, PendingStep> = BTreeMap::new();
    let mut attention: Vec<ReportAttention> = Vec::new();
    for event in events {
        let TaskEventKind::Known(kind) = &event.kind else {
            continue;
        };
        note_kind(kind, &mut steps, &mut attention);
    }
    Ok(DeliveryReportRecord {
        goal: goal.to_owned(),
        status: view.status,
        detail: view.detail.clone(),
        summary: report_summary(reply),
        attention,
        steps: finish_steps(steps),
    })
}

fn finish_steps(steps: BTreeMap<String, PendingStep>) -> Vec<ReportStep> {
    steps
        .into_values()
        .map(|step| ReportStep {
            name: step.name,
            args_summary: step.args_summary,
            outcome: step.outcome,
            evidence_ids: step.evidence_ids,
            verify_source: step.verify_source,
        })
        .collect()
}

fn pending_step(name: &str, args_summary: &str) -> PendingStep {
    PendingStep {
        name: name.to_owned(),
        args_summary: args_summary.to_owned(),
        outcome: ReportOutcome::Completed,
        evidence_ids: Vec::new(),
        verify_source: None,
    }
}

fn note_kind(
    kind: &KnownTaskEventKind,
    steps: &mut BTreeMap<String, PendingStep>,
    attention: &mut Vec<ReportAttention>,
) {
    note_attention(kind, attention);
    note_step(kind, steps);
}

fn note_attention(kind: &KnownTaskEventKind, attention: &mut Vec<ReportAttention>) {
    if let KnownTaskEventKind::ToolFailed {
        name,
        args_summary,
        error,
        ..
    } = kind
    {
        attention.push(ReportAttention {
            tool: name.clone(),
            args_summary: args_summary.clone(),
            error: error.clone(),
        });
    }
}

fn note_step(kind: &KnownTaskEventKind, steps: &mut BTreeMap<String, PendingStep>) {
    match kind {
        KnownTaskEventKind::ToolIntent {
            call_id,
            name,
            args_summary,
            ..
        } => {
            steps.insert(call_id.as_str(), pending_step(name, args_summary));
        }
        KnownTaskEventKind::ToolFailed { call_id, .. } => {
            note_outcome(steps, call_id, ReportOutcome::Failed);
        }
        KnownTaskEventKind::ToolCompleted { call_id, .. } => {
            note_outcome(steps, call_id, ReportOutcome::Completed);
        }
        KnownTaskEventKind::ActionVerified {
            call_id,
            source,
            evidence_ids,
        } => note_verified(steps, call_id, *source, evidence_ids),
        KnownTaskEventKind::TaskCreated { .. }
        | KnownTaskEventKind::TaskCancelled { .. }
        | KnownTaskEventKind::TaskStopped
        | KnownTaskEventKind::Wake { .. }
        | KnownTaskEventKind::GoalPaused { .. }
        | KnownTaskEventKind::ComputerOffline
        | KnownTaskEventKind::ModelUnreachable
        | KnownTaskEventKind::Stalled { .. }
        | KnownTaskEventKind::TurnStarted { .. }
        | KnownTaskEventKind::TurnEnded { .. }
        | KnownTaskEventKind::ContextBuilt { .. }
        | KnownTaskEventKind::ModelAsked { .. }
        | KnownTaskEventKind::ModelAnswer { .. }
        | KnownTaskEventKind::ModelUsage { .. }
        | KnownTaskEventKind::ModelSwapped { .. }
        | KnownTaskEventKind::ToolStarted { .. }
        | KnownTaskEventKind::ToolNotRun { .. }
        | KnownTaskEventKind::GateResult { .. }
        | KnownTaskEventKind::EvidenceAdded { .. }
        | KnownTaskEventKind::OutcomeUnknown { .. }
        | KnownTaskEventKind::Reconciled { .. }
        | KnownTaskEventKind::CriterionPassed { .. }
        | KnownTaskEventKind::CriterionFailed { .. }
        | KnownTaskEventKind::AcceptanceUnavailable
        | KnownTaskEventKind::ClaimChecked { .. }
        | KnownTaskEventKind::BudgetUsage { .. }
        | KnownTaskEventKind::BudgetLimit { .. }
        | KnownTaskEventKind::CardShown { .. }
        | KnownTaskEventKind::CardAnswered { .. }
        | KnownTaskEventKind::QuestionShown { .. }
        | KnownTaskEventKind::QuestionAnswered { .. }
        | KnownTaskEventKind::DeviceReport { .. }
        | KnownTaskEventKind::DeliveryReport { .. }
        | KnownTaskEventKind::DoneWhenChange { .. }
        | KnownTaskEventKind::DoneWhenAppliedByUser { .. } => {}
    }
}

fn note_outcome(
    steps: &mut BTreeMap<String, PendingStep>,
    call_id: &CallId,
    outcome: ReportOutcome,
) {
    if let Some(step) = steps.get_mut(&call_id.as_str()) {
        step.outcome = outcome;
    }
}

fn note_verified(
    steps: &mut BTreeMap<String, PendingStep>,
    call_id: &CallId,
    source: CheckSource,
    evidence_ids: &[anybot_foundation::EvidenceId],
) {
    if let Some(step) = steps.get_mut(&call_id.as_str()) {
        step.evidence_ids = evidence_ids.to_vec();
        step.verify_source = Some(source);
    }
}
