use crate::action::ActionSubject;
use crate::evidence::{ActionContext, build_evidence};
use crate::pass::Pass;
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

pub(crate) fn verify(
    pass: &Pass,
    subject: &ActionSubject,
    result: &ActualResult,
    computer: &dyn GatedComputer,
) -> Option<VerificationRecord> {
    match subject {
        ActionSubject::WriteFile { path, content } => {
            Some(reread_verification(pass, &path.raw, content, computer))
        }
        ActionSubject::Command { .. } => Some(exit_code_verification(result)),
        ActionSubject::ReadFile { .. }
        | ActionSubject::PathCheck { .. }
        | ActionSubject::Capabilities => None,
    }
}

fn reread_verification(
    pass: &Pass,
    path: &str,
    content: &str,
    computer: &dyn GatedComputer,
) -> VerificationRecord {
    match computer.read_file(pass, path) {
        Ok(ActualResult::Read(outcome)) => VerificationRecord {
            result: digest_match(outcome.digest.hex(), content),
            source: JudgementSource::ProgramCheck,
            detail: format!("reread digest of {path}"),
        },
        Ok(other) => VerificationRecord {
            result: VerificationResult::Failed,
            source: JudgementSource::ProgramCheck,
            detail: format!("reread of {path} returned {other:?}"),
        },
        Err(rejection) => VerificationRecord {
            result: VerificationResult::Failed,
            source: JudgementSource::ProgramCheck,
            detail: rejection.reason.clone(),
        },
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
    pass: &Pass,
    subject: &ActionSubject,
    computer: &dyn GatedComputer,
) -> Result<ActualResult, OpRejection> {
    match subject {
        ActionSubject::Command { line, timeout_ms } => {
            computer.run_command(pass, line, *timeout_ms)
        }
        ActionSubject::ReadFile { path } => computer.read_file(pass, &path.raw),
        ActionSubject::WriteFile { path, content } => computer.write_file(pass, &path.raw, content),
        ActionSubject::PathCheck { path } => computer.path_info(pass, &path.raw),
        ActionSubject::Capabilities => computer.capabilities(pass),
    }
}
