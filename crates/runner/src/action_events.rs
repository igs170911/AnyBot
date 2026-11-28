use anybot_foundation::{CallId, EvidenceId};
use anybot_protocol::{
    CheckSource, EvidenceRecord, GateRecord, JudgementSource, KnownTaskEventKind, ToolEffect,
    ToolRequest,
};
use anybot_trust::{ActionRun, CODE_TURN_COMPLETED, CODE_TURN_REJECTED};

use crate::error::TurnError;
use crate::runner::{TurnCtx, TurnRunner};
use crate::view::ActionView;

pub(crate) async fn record_gate(
    runner: &TurnRunner,
    ctx: &TurnCtx,
    call_id: CallId,
    gate: &GateRecord,
) -> Result<(), TurnError> {
    runner
        .append_task_event(
            &ctx.task,
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
) -> Result<(), TurnError> {
    if gate.outcome == anybot_protocol::GateOutcome::Allow {
        runner
            .append_task_event(
                &ctx.task,
                KnownTaskEventKind::ToolIntent {
                    call_id: request.call_id,
                    name: request.tool.clone(),
                    args_summary: crate::action::args_json(&request.arguments)?,
                    level: level_of(&request.tool),
                },
            )
            .await?;
    }
    runner
        .append_task_event(
            &ctx.task,
            KnownTaskEventKind::ToolStarted {
                call_id: request.call_id,
            },
        )
        .await
        .map(|_| ())
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
    let text = view.render(&run.model)?;
    let json = crate::action::args_json(&request.arguments)?;
    if run.summary.status.code == CODE_TURN_REJECTED {
        return Ok(());
    }
    if run.summary.status.code == CODE_TURN_COMPLETED {
        let raw_id = store_output(runner, view).await?;
        let summary = with_body(with_output(&text, &run.evidence), view);
        runner
            .append_task_event(
                &ctx.task,
                KnownTaskEventKind::ToolCompleted {
                    call_id: request.call_id,
                    summary,
                    raw_id,
                },
            )
            .await
            .map(|_| ())
    } else {
        let error = with_output(&text, &run.evidence);
        runner
            .append_task_event(
                &ctx.task,
                KnownTaskEventKind::ToolFailed {
                    call_id: request.call_id,
                    name: request.tool.clone(),
                    args_summary: json,
                    error,
                },
            )
            .await
            .map(|_| ())
    }
}

fn with_output(text: &str, evidence: &[EvidenceRecord]) -> String {
    for record in evidence {
        if let anybot_protocol::EvidencePayload::Executed {
            output_head: head, ..
        } = &record.payload
            && !head.is_empty()
        {
            return format!(
                "{text}
{head}"
            );
        }
    }
    text.to_owned()
}

fn with_body(text: String, view: &ActionView) -> String {
    match view.take_body() {
        Some(body) if !body.is_empty() => format!("{text}\n{body}"),
        _ => text,
    }
}

async fn store_output(runner: &TurnRunner, view: &ActionView) -> Result<Option<String>, TurnError> {
    let Some(output) = view.take_output() else {
        return Ok(None);
    };
    let bytes = output.into_bytes();
    let digest = anybot_foundation::ContentDigest::sha256(&bytes);
    runner.raw.put(bytes).await.map_err(TurnError::Raw)?;
    Ok(Some(digest.hex().to_owned()))
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
            .append_task_event(
                &ctx.task,
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
        .append_task_event(
            &ctx.task,
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
