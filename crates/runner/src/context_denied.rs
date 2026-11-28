use anybot_context::{StepOutcomeKind, StepRecord};
use anybot_foundation::{CallId, EventId, TaskId};
use anybot_messages::Lang;
use anybot_protocol::{
    EvidenceKind, EvidencePayload, EvidenceRecord, GateNo, GateRecord, KnownTaskEventKind,
    NotRunReason, ReasonPayload, ToolArgs, ToolEffect,
};

use crate::context_input::known_of;
use crate::context_steps::results_of;
use crate::error::TurnError;
use crate::record::Stored;

pub(crate) struct CallResult {
    pub seq: u64,
    pub event_id: EventId,
    pub outcome: StepOutcomeKind,
    pub text: String,
    pub records: Vec<EvidenceRecord>,
}

impl CallResult {
    pub(crate) fn of(seq: u64, event_id: EventId, outcome: StepOutcomeKind, text: String) -> Self {
        CallResult {
            seq,
            event_id,
            outcome,
            text,
            records: Vec::new(),
        }
    }

    pub(crate) fn exit_code(&self) -> Option<i64> {
        for record in &self.records {
            if let EvidencePayload::Executed {
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

pub(crate) fn not_run_word(reason: NotRunReason) -> String {
    match reason {
        NotRunReason::NotWired => "not-wired".to_owned(),
        NotRunReason::OverBudget => "over-budget".to_owned(),
        NotRunReason::PreviousRefused => "previous-refused".to_owned(),
        NotRunReason::Stopped => "stopped".to_owned(),
        NotRunReason::ComputerOffline => "computer-unreachable".to_owned(),
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
    lang: Lang,
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
    lang: Lang,
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
    let exit_code = found.exit_code();
    let error_at = anybot_context::find_error_line(&content, exit_code);
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
        exit_code,
        error_at,
    })
}

fn level_of(tool: &str) -> ToolEffect {
    match anybot_tools::level_of(tool) {
        anybot_tools::ToolLevel::ReadOnly => ToolEffect::ReadOnly,
        anybot_tools::ToolLevel::Reversible => ToolEffect::Repeatable,
        anybot_tools::ToolLevel::Irreversible => ToolEffect::Irreversible,
    }
}

fn args_text(args: &ToolArgs) -> Result<(String, serde_json::Value), TurnError> {
    match args {
        ToolArgs::Json { value } => Ok((serde_json::to_string(value)?, value.clone())),
        ToolArgs::Broken { raw } => Ok((raw.clone(), serde_json::Value::String(raw.clone()))),
    }
}

pub(crate) fn collect_gate(gates: &mut Vec<(GateNo, ReasonPayload)>, kind: &KnownTaskEventKind) {
    let KnownTaskEventKind::EvidenceAdded { record } = kind else {
        return;
    };
    if record.kind != EvidenceKind::Rejected {
        return;
    }
    if let EvidencePayload::Rejected { reason } = &record.payload {
        gates.push((record.gate.gate, reason.clone()));
    }
}

fn denied_fallback(lang: Lang, gate: &GateRecord) -> Result<String, TurnError> {
    crate::view::render_text(
        lang,
        &anybot_trust::OutcomeText::of(
            "gate-denied-by-rule",
            vec![("rule".to_owned(), gate.rule.clone())],
        ),
    )
}

pub(crate) fn denied_text(
    lang: Lang,
    gate: &GateRecord,
    gates: &[(GateNo, ReasonPayload)],
) -> Result<String, TurnError> {
    let Some((_, reason)) = gates.iter().find(|(no, _)| *no == gate.gate) else {
        return denied_fallback(lang, gate);
    };
    let Some(params) = wanted_params(lang, reason) else {
        return denied_fallback(lang, gate);
    };
    let rendered = crate::view::render_text(
        lang,
        &anybot_trust::OutcomeText::of(reason.code.as_str(), params),
    );
    rendered.or_else(|_| denied_fallback(lang, gate))
}

fn wanted_params(lang: Lang, reason: &ReasonPayload) -> Option<Vec<(String, String)>> {
    let renamed = normalize_params(&reason.code, &reason.params);
    let code = anybot_messages::MessageCode::from_code(&reason.code)?;
    let mut catalog = anybot_messages::Catalog::new();
    let names = catalog.param_names(lang, &code).ok()?;
    Some(
        renamed
            .into_iter()
            .filter(|(key, _)| names.iter().any(|name| name == key))
            .collect(),
    )
}

fn normalize_params(code: &str, params: &[(String, String)]) -> Vec<(String, String)> {
    let Some(wanted) = (code == "gate-command-denied").then(|| "command".to_owned()) else {
        return params.to_vec();
    };
    let mut out: Vec<(String, String)> = Vec::new();
    for (key, value) in params {
        let name = if key == "token" {
            wanted.clone()
        } else {
            key.clone()
        };
        out.push((name, value.clone()));
    }
    out
}
