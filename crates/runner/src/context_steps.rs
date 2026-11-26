use std::collections::BTreeMap;

use anybot_context::{StepOutcomeKind, StepRecord};
use anybot_foundation::{CallId, TaskId};
use anybot_protocol::{KnownTaskEventKind, ToolArgs};

use crate::context_input::{known_of, row_event_id};
use crate::error::TurnError;
use crate::record::Stored;

fn results_of(
    stored: &[Stored],
    task: &TaskId,
    lang: anybot_messages::Lang,
) -> BTreeMap<CallId, CallResult> {
    let mut map: BTreeMap<CallId, CallResult> = BTreeMap::new();
    let mut pool: BTreeMap<anybot_foundation::EvidenceId, anybot_protocol::EvidenceRecord> =
        BTreeMap::new();
    for row in stored {
        let Some(kind) = known_of(row, task) else {
            continue;
        };
        let Some(event_id) = row_event_id(row) else {
            continue;
        };
        apply_row(&mut map, &mut pool, kind, row.stream_seq, event_id, lang);
    }
    map
}

fn apply_row(
    map: &mut BTreeMap<CallId, CallResult>,
    pool: &mut BTreeMap<anybot_foundation::EvidenceId, anybot_protocol::EvidenceRecord>,
    kind: &KnownTaskEventKind,
    seq: u64,
    event_id: anybot_foundation::EventId,
    lang: anybot_messages::Lang,
) {
    if let Ok(Some(entry)) = result_entry(kind, seq, event_id, lang) {
        map.insert(entry.0, entry.1);
        return;
    }
    if let KnownTaskEventKind::EvidenceAdded { record } = kind {
        pool.insert(record.id, (**record).clone());
        return;
    }
    if let KnownTaskEventKind::ActionVerified {
        call_id,
        evidence_ids,
        ..
    } = kind
    {
        attach_verified(map, pool, *call_id, evidence_ids);
    }
}

fn result_entry(
    kind: &KnownTaskEventKind,
    seq: u64,
    event_id: anybot_foundation::EventId,
    lang: anybot_messages::Lang,
) -> Result<Option<(CallId, CallResult)>, TurnError> {
    if let KnownTaskEventKind::GateResult { call_id, gate } = kind {
        if gate.outcome != anybot_protocol::GateOutcome::Deny {
            return Ok(None);
        }
        let text = crate::view::render_text(
            lang,
            &anybot_trust::OutcomeText::of(
                "gate-denied-by-rule",
                vec![("rule".to_owned(), gate.rule.clone())],
            ),
        )?;
        let found = CallResult::of(seq, event_id, StepOutcomeKind::Completed, text);
        return Ok(Some((*call_id, found)));
    }
    if let KnownTaskEventKind::ToolCompleted {
        call_id, summary, ..
    } = kind
    {
        let found = CallResult::of(seq, event_id, StepOutcomeKind::Completed, summary.clone());
        return Ok(Some((*call_id, found)));
    }
    if let KnownTaskEventKind::ToolFailed { call_id, error, .. } = kind {
        let found = CallResult::of(seq, event_id, StepOutcomeKind::Failed, error.clone());
        return Ok(Some((*call_id, found)));
    }
    if let KnownTaskEventKind::ToolNotRun { call_id, reason } = kind {
        let found = CallResult::of(
            seq,
            event_id,
            StepOutcomeKind::Unknown,
            not_run_word(*reason),
        );
        return Ok(Some((*call_id, found)));
    }
    Ok(None)
}

struct CallResult {
    seq: u64,
    event_id: anybot_foundation::EventId,
    outcome: StepOutcomeKind,
    text: String,
    records: Vec<anybot_protocol::EvidenceRecord>,
}

impl CallResult {
    fn of(
        seq: u64,
        event_id: anybot_foundation::EventId,
        outcome: StepOutcomeKind,
        text: String,
    ) -> Self {
        CallResult {
            seq,
            event_id,
            outcome,
            text,
            records: Vec::new(),
        }
    }

    fn exit_code(&self) -> Option<i64> {
        for record in &self.records {
            if let anybot_protocol::EvidencePayload::Executed {
                exit_code: Some(code),
                ..
            } = &record.payload
            {
                return Some(i64::from(*code));
            }
        }
        None
    }
}

fn attach_verified(
    map: &mut BTreeMap<CallId, CallResult>,
    pool: &BTreeMap<anybot_foundation::EvidenceId, anybot_protocol::EvidenceRecord>,
    call_id: CallId,
    evidence_ids: &[anybot_foundation::EvidenceId],
) {
    if let Some(entry) = map.get_mut(&call_id) {
        for id in evidence_ids {
            if let Some(record) = pool.get(id) {
                entry.records.push(record.clone());
            }
        }
    }
}

fn not_run_word(reason: anybot_protocol::NotRunReason) -> String {
    match reason {
        anybot_protocol::NotRunReason::NotWired => "not-wired".to_owned(),
        anybot_protocol::NotRunReason::OverBudget => "over-budget".to_owned(),
        anybot_protocol::NotRunReason::PreviousRefused => "previous-refused".to_owned(),
        anybot_protocol::NotRunReason::Stopped => "stopped".to_owned(),
        anybot_protocol::NotRunReason::ComputerOffline => "computer-unreachable".to_owned(),
    }
}

fn answers_of(stored: &[Stored], task: &TaskId) -> Vec<Vec<ToolArgsPair>> {
    stored
        .iter()
        .filter_map(|row| match known_of(row, task) {
            Some(KnownTaskEventKind::ModelAnswer { tool_requests, .. }) => Some(
                tool_requests
                    .iter()
                    .map(|r| ToolArgsPair {
                        call_id: r.call_id,
                        tool: r.tool.clone(),
                        arguments: r.arguments.clone(),
                    })
                    .collect(),
            ),
            Some(_) | None => None,
        })
        .collect()
}

struct ToolArgsPair {
    call_id: CallId,
    tool: String,
    arguments: ToolArgs,
}

pub(crate) fn steps_of(
    stored: &[Stored],
    task: &TaskId,
    lang: anybot_messages::Lang,
) -> Result<Vec<StepRecord>, TurnError> {
    let results = results_of(stored, task, lang);
    let mut steps: Vec<StepRecord> = Vec::new();
    for answer in answers_of(stored, task) {
        for request in answer {
            let Some(found) = results.get(&request.call_id) else {
                return Err(TurnError::UnpairedToolRequest {
                    call_id: request.call_id,
                });
            };
            steps.push(step_record_of(lang, &request, found)?);
        }
    }
    Ok(steps)
}

fn step_record_of(
    lang: anybot_messages::Lang,
    request: &ToolArgsPair,
    found: &CallResult,
) -> Result<StepRecord, TurnError> {
    let (summary, value) = args_text(&request.arguments)?;
    let content = if found.outcome == StepOutcomeKind::Unknown {
        crate::view::render_text(
            lang,
            &anybot_trust::OutcomeText::of(
                "turn-not-executed",
                vec![("reason".to_owned(), found.text.clone())],
            ),
        )?
    } else {
        found.text.clone()
    };
    Ok(StepRecord {
        seq: found.seq,
        call_id: request.call_id,
        tool: request.tool.clone(),
        args_summary: summary,
        arguments: value,
        level: level_of(&request.tool),
        unconfirmed: false,
        outcome: found.outcome,
        source_ref: found.event_id.as_str(),
        content,
        exit_code: found.exit_code(),
        error_at: Vec::new(),
    })
}

fn level_of(tool: &str) -> anybot_protocol::ToolEffect {
    match anybot_tools::level_of(tool) {
        anybot_tools::ToolLevel::ReadOnly => anybot_protocol::ToolEffect::ReadOnly,
        anybot_tools::ToolLevel::Reversible => anybot_protocol::ToolEffect::Repeatable,
        anybot_tools::ToolLevel::Irreversible => anybot_protocol::ToolEffect::Irreversible,
    }
}

fn args_text(args: &ToolArgs) -> Result<(String, serde_json::Value), TurnError> {
    match args {
        ToolArgs::Json { value } => Ok((serde_json::to_string(value)?, value.clone())),
        ToolArgs::Broken { raw } => Ok((raw.clone(), serde_json::Value::String(raw.clone()))),
    }
}
