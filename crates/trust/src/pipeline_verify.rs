use crate::action::{ActionKey, ActionSubject, PathView};
use crate::evidence::{ActionContext, build_evidence};
use crate::op_ticket::OpTicket;
use crate::runner::{GatedComputer, OpRejection};
use anybot_foundation::{Clock, ContentDigest};
use anybot_protocol::{
    ActionOutcome, ActualResult, EvidenceRecord, GateRecord, JudgementSource, VerificationRecord,
    VerificationResult,
};

pub(crate) fn verified_evidence(
    context: &ActionContext,
    gate: &GateRecord,
    result: &ActualResult,
    verification: Option<VerificationRecord>,
    clock: &dyn Clock,
) -> EvidenceRecord {
    let mut evidence = build_evidence(context, gate, result, clock);
    evidence.outcome = completed_outcome(result, verification.as_ref());
    evidence.verification = verification;
    evidence
}

pub(crate) fn completed_outcome(
    result: &ActualResult,
    verification: Option<&VerificationRecord>,
) -> ActionOutcome {
    if let ActualResult::Command(outcome) = result
        && outcome.exit_code != Some(0)
    {
        return ActionOutcome::Failed;
    }
    if verification.is_some_and(|v| v.result == VerificationResult::Failed) {
        return ActionOutcome::Failed;
    }
    ActionOutcome::Completed
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct VerificationPlan {
    pub kind: crate::op_ticket::OpKind,
    pub tool: String,
    pub path: String,
    pub content: String,
    pub computer: String,
    pub caps: anybot_protocol::ComputerCaps,
    pub workdir: String,
    pub gate: GateRecord,
}

pub(crate) fn plan_of(ticket: &OpTicket) -> VerificationPlan {
    VerificationPlan {
        kind: ticket.kind(),
        tool: ticket.key().parts().0.to_owned(),
        path: ticket.path().unwrap_or("").to_owned(),
        content: ticket.content().unwrap_or("").to_owned(),
        computer: ticket.computer().to_owned(),
        caps: ticket.caps().clone(),
        workdir: ticket.workdir().to_owned(),
        gate: ticket.gate().clone(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum VerificationTarget {
    RereadContent,
    AfterDigest,
}

pub(crate) fn verify(
    plan: &VerificationPlan,
    result: &ActualResult,
    computer: &dyn GatedComputer,
    clock: &dyn Clock,
) -> Option<VerificationRecord> {
    match plan.kind {
        crate::op_ticket::OpKind::Write => Some(reread_verification(
            plan,
            result,
            computer,
            clock,
            VerificationTarget::RereadContent,
        )),
        crate::op_ticket::OpKind::Edit => Some(reread_verification(
            plan,
            result,
            computer,
            clock,
            VerificationTarget::AfterDigest,
        )),
        crate::op_ticket::OpKind::Run => Some(exit_code_verification(result)),
        crate::op_ticket::OpKind::Read
        | crate::op_ticket::OpKind::ListDir
        | crate::op_ticket::OpKind::Search
        | crate::op_ticket::OpKind::Path
        | crate::op_ticket::OpKind::Caps => None,
    }
}

fn reread_verification(
    plan: &VerificationPlan,
    result: &ActualResult,
    computer: &dyn GatedComputer,
    clock: &dyn Clock,
    target: VerificationTarget,
) -> VerificationRecord {
    let path = plan.path.clone();
    let subject = ActionSubject::ReadFile {
        path: PathView::raw(&path),
    };
    let key = ActionKey::file(&plan.tool, "read", &path);
    let ticket = OpTicket::issue(
        &subject,
        key,
        plan.gate.clone(),
        plan.computer.as_str(),
        &plan.caps,
        &plan.workdir,
        clock.now_ms(),
    );
    match computer.read_file(ticket) {
        Ok(ActualResult::Read(outcome)) => match target {
            VerificationTarget::RereadContent => VerificationRecord {
                result: digest_match(outcome.digest.hex(), &plan.content),
                source: JudgementSource::ProgramCheck,
                detail: format!("reread digest of {path}"),
            },
            VerificationTarget::AfterDigest => VerificationRecord {
                result: after_digest_match(&outcome, result),
                source: JudgementSource::ProgramCheck,
                detail: format!("reread digest of {path} against the written digest"),
            },
        },
        Ok(other) => failed_check(format!("reread of {path} returned {other:?}")),
        Err(rejection) => failed_check(rejection.reason.clone()),
    }
}

fn after_digest_match(
    reread: &anybot_protocol::ReadOutcome,
    result: &ActualResult,
) -> VerificationResult {
    let ActualResult::Write(outcome) = result else {
        return VerificationResult::Failed;
    };
    if reread.digest == outcome.after {
        VerificationResult::Passed
    } else {
        VerificationResult::Failed
    }
}

fn failed_check(detail: String) -> VerificationRecord {
    VerificationRecord {
        result: VerificationResult::Failed,
        source: JudgementSource::ProgramCheck,
        detail,
    }
}

fn digest_match(hex: &str, content: &str) -> VerificationResult {
    if hex == ContentDigest::sha256(content.as_bytes()).hex() {
        VerificationResult::Passed
    } else {
        VerificationResult::Failed
    }
}

fn exit_code_verification(result: &ActualResult) -> VerificationRecord {
    match result {
        ActualResult::Command(outcome) => VerificationRecord {
            result: if outcome.exit_code.is_some() {
                VerificationResult::Passed
            } else {
                VerificationResult::Failed
            },
            source: JudgementSource::ProgramCheck,
            detail: format!("exit code {}", exit_word(outcome.exit_code)),
        },
        ActualResult::Write(_)
        | ActualResult::Read(_)
        | ActualResult::List(_)
        | ActualResult::Search(_)
        | ActualResult::Path(_)
        | ActualResult::Caps(_) => VerificationRecord {
            result: VerificationResult::NothingToCheck,
            source: JudgementSource::Rule,
            detail: "no exit code in this result".to_owned(),
        },
    }
}

fn exit_word(code: Option<i32>) -> String {
    match code {
        Some(value) => value.to_string(),
        None => "missing".to_owned(),
    }
}

pub(crate) fn perform(
    ticket: OpTicket,
    computer: &dyn GatedComputer,
) -> Result<ActualResult, OpRejection> {
    match ticket.kind() {
        crate::op_ticket::OpKind::Run => computer.run_command(ticket),
        crate::op_ticket::OpKind::Read => computer.read_file(ticket),
        crate::op_ticket::OpKind::ListDir => computer.list_dir(ticket),
        crate::op_ticket::OpKind::Search => computer.search(ticket),
        crate::op_ticket::OpKind::Write => computer.write_file(ticket),
        crate::op_ticket::OpKind::Edit => computer.edit_file(ticket),
        crate::op_ticket::OpKind::Path => computer.path_info(ticket),
        crate::op_ticket::OpKind::Caps => computer.capabilities(ticket),
    }
}
