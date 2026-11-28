use anybot_foundation::{CallId, EvidenceId};
use anybot_protocol::{
    CheckSource, EvidenceRecord, GateRecord, JudgementSource, KnownTaskEventKind, ToolEffect,
    ToolRequest,
};
use anybot_trust::{ActionRun, CODE_TURN_COMPLETED, CODE_TURN_REJECTED};

use crate::error::TurnError;
use crate::record::{Appended, Halt};
use crate::runner::{TurnCtx, TurnRunner};
use crate::view::ActionView;

pub(crate) async fn record_gate(
    runner: &TurnRunner,
    ctx: &TurnCtx,
    call_id: CallId,
    gate: &GateRecord,
) -> Result<(), TurnError> {
    runner
        .append_turn_event(
            ctx,
            KnownTaskEventKind::GateResult {
                call_id,
                gate: gate.clone(),
            },
        )
        .await
        .map(|_| ())
}

pub(crate) async fn record_intent_start(
    runner: &TurnRunner,
    ctx: &TurnCtx,
    request: &ToolRequest,
    gate: &GateRecord,
) -> Result<Option<Halt>, TurnError> {
    if gate.outcome == anybot_protocol::GateOutcome::Allow
        && let Appended::Halted(halt) = runner
            .append_turn_unless_halted(
                ctx,
                KnownTaskEventKind::ToolIntent {
                    call_id: request.call_id,
                    name: request.tool.clone(),
                    args_summary: crate::action::args_json(&request.arguments)?,
                    level: level_of(&request.tool),
                },
            )
            .await?
    {
        return Ok(Some(halt));
    }
    if let Appended::Halted(halt) = runner
        .append_turn_unless_halted(
            ctx,
            KnownTaskEventKind::ToolStarted {
                call_id: request.call_id,
            },
        )
        .await?
    {
        return Ok(Some(halt));
    }
    Ok(None)
}

fn level_of(tool: &str) -> ToolEffect {
    match anybot_tools::level_of(tool) {
        anybot_tools::ToolLevel::ReadOnly => ToolEffect::ReadOnly,
        anybot_tools::ToolLevel::Reversible => ToolEffect::Repeatable,
        anybot_tools::ToolLevel::Irreversible => ToolEffect::Irreversible,
    }
}

pub(crate) async fn record_result(
    runner: &TurnRunner,
    ctx: &TurnCtx,
    request: &ToolRequest,
    run: &ActionRun,
    view: &ActionView,
) -> Result<(), TurnError> {
    record_result_noted(runner, ctx, request, run, view, None).await
}

pub(crate) async fn record_result_noted(
    runner: &TurnRunner,
    ctx: &TurnCtx,
    request: &ToolRequest,
    run: &ActionRun,
    view: &ActionView,
    note: Option<String>,
) -> Result<(), TurnError> {
    let rendered = view.render(&run.model)?;
    let json = crate::action::args_json(&request.arguments)?;
    if run.summary.status.code == CODE_TURN_REJECTED {
        return Ok(());
    }
    let captured = view.take_output();
    let digest = store_output(runner, captured.as_ref()).await?;
    let raw_id = digest.as_ref().map(|d| d.hex().to_owned());
    let shell = match &captured {
        Some(cap) => {
            crate::view_text::shell_body(runner.lang, &cap.output, cap.exit_code, digest.as_ref())?
        }
        None => None,
    };
    let body = combine_body(
        &run.model.code,
        rendered,
        shell.or_else(|| view.take_body()),
    );
    let summary = prefix_note(note, body);
    if run.summary.status.code == CODE_TURN_COMPLETED {
        let summary = with_computer_prefix(runner, &summary);
        runner
            .append_turn_event(
                ctx,
                KnownTaskEventKind::ToolCompleted {
                    call_id: request.call_id,
                    summary,
                    raw_id,
                },
            )
            .await
            .map(|_| ())
    } else {
        runner
            .append_turn_event(
                ctx,
                KnownTaskEventKind::ToolFailed {
                    call_id: request.call_id,
                    name: request.tool.clone(),
                    args_summary: json,
                    error: summary,
                },
            )
            .await
            .map(|_| ())
    }
}

pub(crate) fn computer_label_prefix(runner: &TurnRunner) -> String {
    if runner.gated.declared_caps().owner != anybot_protocol::ComputerOwner::User {
        return String::new();
    }
    let name = runner.gated.name();
    format!("[{name}] ")
}

pub(crate) fn without_computer_label(text: &str, prefix: &str) -> String {
    if prefix.is_empty() {
        return text.to_owned();
    }
    if let Some(rest) = text.strip_prefix(prefix) {
        rest.to_owned()
    } else {
        text.to_owned()
    }
}

fn with_computer_prefix(runner: &TurnRunner, text: &str) -> String {
    let prefix = computer_label_prefix(runner);
    if prefix.is_empty() || text.starts_with(&prefix) {
        text.to_owned()
    } else {
        format!("{prefix}{text}")
    }
}

fn prefix_note(note: Option<String>, body: String) -> String {
    match note {
        Some(note) => format!("{note} {body}"),
        None => body,
    }
}

fn combine_body(code: &str, text: String, body: Option<String>) -> String {
    match body {
        Some(content) if !content.is_empty() && code == anybot_trust::CODE_TOOL_EMPTY => content,
        Some(content) if !content.is_empty() => format!("{text}\n{content}"),
        _ => text,
    }
}

async fn store_output(
    runner: &TurnRunner,
    captured: Option<&crate::view::CapturedShell>,
) -> Result<Option<anybot_foundation::ContentDigest>, TurnError> {
    let Some(cap) = captured else {
        return Ok(None);
    };
    let bytes = cap.output.as_bytes().to_vec();
    let digest = runner.raw.put(bytes).await.map_err(TurnError::Raw)?;
    Ok(Some(digest))
}

pub(crate) async fn record_evidence(
    runner: &TurnRunner,
    ctx: &TurnCtx,
    records: &[EvidenceRecord],
) -> Result<Vec<EvidenceId>, TurnError> {
    let mut ids: Vec<EvidenceId> = Vec::new();
    for record in records {
        let short = ctx.next_evidence();
        let mut record = record.clone();
        record.short = anybot_protocol::EvidenceShortId(short);
        ids.push(record.id);
        runner
            .append_turn_event(
                ctx,
                KnownTaskEventKind::EvidenceAdded {
                    record: Box::new(record),
                },
            )
            .await?;
    }
    Ok(ids)
}

pub(crate) async fn record_verified(
    runner: &TurnRunner,
    ctx: &TurnCtx,
    call_id: CallId,
    run: &ActionRun,
    ids: Vec<EvidenceId>,
) -> Result<(), TurnError> {
    let source = verification_source(&run.evidence);
    runner
        .append_turn_event(
            ctx,
            KnownTaskEventKind::ActionVerified {
                call_id,
                source,
                evidence_ids: ids,
            },
        )
        .await
        .map(|_| ())
}

fn verification_source(evidence: &[EvidenceRecord]) -> CheckSource {
    evidence
        .iter()
        .find_map(|record| record.verification.as_ref())
        .map_or(CheckSource::Program, |v| check_source_of(v.source))
}

fn check_source_of(source: JudgementSource) -> CheckSource {
    match source {
        JudgementSource::ProgramCheck
        | JudgementSource::RuleTimeout
        | JudgementSource::Ungated
        | JudgementSource::ForwardedUnverified => CheckSource::Program,
        JudgementSource::Rule | JudgementSource::You | JudgementSource::AlwaysApprove => {
            CheckSource::Rule
        }
        JudgementSource::ModelJudgement => CheckSource::Model,
    }
}
