use anybot_foundation::EvidenceId;
use anybot_protocol::{GateOutcome, KnownTaskEventKind, TaskEvent, TaskEventKind};

pub fn one_result_each(kinds: &[KnownTaskEventKind]) -> Result<(), String> {
    let mut intents: Vec<String> = Vec::new();
    let mut results: Vec<String> = Vec::new();
    for kind in kinds {
        push_ids(kind, &mut intents, &mut results);
    }
    for id in &intents {
        let hits = results
            .iter()
            .filter(|got| got.as_str() == id.as_str())
            .count();
        if hits != 1 {
            return Err(format!("intent {id} results {hits}"));
        }
    }
    Ok(())
}

fn push_ids(kind: &KnownTaskEventKind, intents: &mut Vec<String>, results: &mut Vec<String>) {
    match kind {
        KnownTaskEventKind::ToolIntent { call_id, .. } => intents.push(call_id.as_str()),
        KnownTaskEventKind::ToolCompleted { call_id, .. }
        | KnownTaskEventKind::ToolFailed { call_id, .. }
        | KnownTaskEventKind::ToolNotRun { call_id, .. } => results.push(call_id.as_str()),
        KnownTaskEventKind::GateResult { call_id, gate } => {
            if gate.outcome == GateOutcome::Deny {
                results.push(call_id.as_str());
            }
        }
        KnownTaskEventKind::TaskCreated { .. }
        | KnownTaskEventKind::DoneWhenChange { .. }
        | KnownTaskEventKind::DoneWhenAppliedByUser { .. }
        | KnownTaskEventKind::CriterionPassed { .. }
        | KnownTaskEventKind::CriterionFailed { .. }
        | KnownTaskEventKind::AcceptanceUnavailable
        | KnownTaskEventKind::ClaimChecked { .. }
        | KnownTaskEventKind::TaskCancelled { .. }
        | KnownTaskEventKind::TaskStopped
        | KnownTaskEventKind::Wake { .. }
        | KnownTaskEventKind::GoalPaused { .. }
        | KnownTaskEventKind::Stalled { .. }
        | KnownTaskEventKind::ModelUnreachable
        | KnownTaskEventKind::ComputerOffline
        | KnownTaskEventKind::TurnStarted { .. }
        | KnownTaskEventKind::TurnEnded { .. }
        | KnownTaskEventKind::ContextBuilt { .. }
        | KnownTaskEventKind::ModelAsked { .. }
        | KnownTaskEventKind::ModelAnswer { .. }
        | KnownTaskEventKind::ModelUsage { .. }
        | KnownTaskEventKind::ModelSwapped { .. }
        | KnownTaskEventKind::ToolStarted { .. }
        | KnownTaskEventKind::OutcomeUnknown { .. }
        | KnownTaskEventKind::Reconciled { .. }
        | KnownTaskEventKind::ActionVerified { .. }
        | KnownTaskEventKind::EvidenceAdded { .. }
        | KnownTaskEventKind::BudgetUsage { .. }
        | KnownTaskEventKind::BudgetLimit { .. }
        | KnownTaskEventKind::CardShown { .. }
        | KnownTaskEventKind::CardAnswered { .. }
        | KnownTaskEventKind::DeviceReport { .. } => {}
    }
}

pub fn exec_order(events: &[TaskEvent]) -> Result<(), String> {
    let mut evidence_seq: std::collections::BTreeMap<EvidenceId, u64> =
        std::collections::BTreeMap::new();
    for event in events {
        let TaskEventKind::Known(KnownTaskEventKind::EvidenceAdded { record }) = &event.kind else {
            continue;
        };
        evidence_seq.insert(record.id, event.seq);
    }
    let mut seqs: Vec<(String, Vec<(u64, String)>)> = Vec::new();
    for event in events {
        let TaskEventKind::Known(kind) = &event.kind else {
            continue;
        };
        push_seq(&mut seqs, kind, event.seq, &evidence_seq);
    }
    for (id, steps) in seqs {
        step_order(&id, &steps)?;
    }
    Ok(())
}

fn push_seq(
    seqs: &mut Vec<(String, Vec<(u64, String)>)>,
    kind: &KnownTaskEventKind,
    seq: u64,
    evidence_seq: &std::collections::BTreeMap<EvidenceId, u64>,
) {
    let Some(id) = order_call_id(kind) else {
        return;
    };
    let mut names: Vec<(u64, String)> = vec![(seq, kind.kind_name().clone())];
    if let KnownTaskEventKind::ActionVerified { evidence_ids, .. } = kind {
        names.pop();
        for ev in evidence_ids {
            if let Some(ev_seq) = evidence_seq.get(ev) {
                names.push((*ev_seq, "evidence_added".to_owned()));
            }
        }
        names.push((seq, "action_verified".to_owned()));
    }
    for entry in seqs.iter_mut() {
        if entry.0 == id {
            entry.1.extend(names);
            return;
        }
    }
    seqs.push((id, names));
}

fn order_call_id(kind: &KnownTaskEventKind) -> Option<String> {
    match kind {
        KnownTaskEventKind::GateResult { call_id, .. }
        | KnownTaskEventKind::ToolIntent { call_id, .. }
        | KnownTaskEventKind::ToolStarted { call_id }
        | KnownTaskEventKind::ToolCompleted { call_id, .. }
        | KnownTaskEventKind::ToolFailed { call_id, .. }
        | KnownTaskEventKind::ActionVerified { call_id, .. } => Some(call_id.as_str()),
        KnownTaskEventKind::TaskCreated { .. }
        | KnownTaskEventKind::DoneWhenChange { .. }
        | KnownTaskEventKind::DoneWhenAppliedByUser { .. }
        | KnownTaskEventKind::CriterionPassed { .. }
        | KnownTaskEventKind::CriterionFailed { .. }
        | KnownTaskEventKind::AcceptanceUnavailable
        | KnownTaskEventKind::ClaimChecked { .. }
        | KnownTaskEventKind::TaskCancelled { .. }
        | KnownTaskEventKind::TaskStopped
        | KnownTaskEventKind::Wake { .. }
        | KnownTaskEventKind::GoalPaused { .. }
        | KnownTaskEventKind::Stalled { .. }
        | KnownTaskEventKind::ModelUnreachable
        | KnownTaskEventKind::ComputerOffline
        | KnownTaskEventKind::TurnStarted { .. }
        | KnownTaskEventKind::TurnEnded { .. }
        | KnownTaskEventKind::ContextBuilt { .. }
        | KnownTaskEventKind::ModelAsked { .. }
        | KnownTaskEventKind::ModelAnswer { .. }
        | KnownTaskEventKind::ModelUsage { .. }
        | KnownTaskEventKind::ModelSwapped { .. }
        | KnownTaskEventKind::ToolNotRun { .. }
        | KnownTaskEventKind::EvidenceAdded { .. }
        | KnownTaskEventKind::OutcomeUnknown { .. }
        | KnownTaskEventKind::Reconciled { .. }
        | KnownTaskEventKind::BudgetUsage { .. }
        | KnownTaskEventKind::BudgetLimit { .. }
        | KnownTaskEventKind::CardShown { .. }
        | KnownTaskEventKind::CardAnswered { .. }
        | KnownTaskEventKind::DeviceReport { .. } => None,
    }
}

fn step_order(id: &str, steps: &[(u64, String)]) -> Result<(), String> {
    let wanted = [
        "gate_result",
        "tool_intent",
        "tool_started",
        "tool_completed",
        "tool_failed",
        "evidence_added",
        "action_verified",
    ];
    let mut ordered: Vec<(u64, String)> = steps.to_vec();
    ordered.sort_by_key(|entry| entry.0);
    let mut last = 0u64;
    for (seq, name) in ordered {
        if !wanted.contains(&name.as_str()) {
            return Err(format!("call {id} extra {name}"));
        }
        if seq <= last {
            return Err(format!("call {id} order {name} seq {seq} last {last}"));
        }
        last = seq;
    }
    Ok(())
}
