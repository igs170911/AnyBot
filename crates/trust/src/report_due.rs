use anybot_protocol::{
    JudgementSource, KnownTaskEventKind, TaskDetail, TaskEvent, TaskEventKind, TaskStatus,
};

use crate::report::ReportFail;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportDue {
    Full,
    Progress,
    None,
}

pub fn report_due(events: &[TaskEvent]) -> Result<ReportDue, ReportFail> {
    let view = anybot_domain::replay(events).map_err(|gap| ReportFail { gap })?;
    if matches!(view.detail, Some(TaskDetail::OverBudget { .. }))
        && view.status == TaskStatus::Stuck
    {
        return Ok(ReportDue::Progress);
    }
    let finished = matches!(
        view.status,
        TaskStatus::Completed | TaskStatus::Unaccepted | TaskStatus::Failed
    );
    let acted =
        view.evidence.actions_taken || !view.criteria.is_empty() || has_automatic_approval(events);
    if finished && acted {
        return Ok(ReportDue::Full);
    }
    Ok(ReportDue::None)
}

fn has_automatic_approval(events: &[TaskEvent]) -> bool {
    events.iter().any(|event| {
        let TaskEventKind::Known(KnownTaskEventKind::GateResult { gate, .. }) = &event.kind else {
            return false;
        };
        matches!(
            gate.source,
            JudgementSource::RuleTimeout | JudgementSource::AlwaysApprove
        )
    })
}
