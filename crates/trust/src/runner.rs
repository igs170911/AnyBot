use anybot_protocol::{
    ActualResult, CommandOutcome, ComputerCaps, ComputerFailure, ComputerFailureKind, PathOutcome,
    ReadOutcome, WriteOutcome,
};

use crate::pass::Pass;

pub trait ComputerRunner: Send + Sync {
    fn capabilities(&self) -> ComputerCaps;

    fn run_command(
        &self,
        command: &str,
        timeout_ms: u64,
    ) -> Result<CommandOutcome, ComputerFailure>;

    fn read_file(&self, path: &str) -> Result<ReadOutcome, ComputerFailure>;

    fn write_file(&self, path: &str, content: &str) -> Result<WriteOutcome, ComputerFailure>;

    fn path_info(&self, path: &str) -> Result<PathOutcome, ComputerFailure>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpRejectionKind {
    PassNotAuthorized,
    Unreachable,
    Operation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpRejection {
    pub kind: OpRejectionKind,
    pub reason: String,
}

impl OpRejection {
    #[must_use]
    pub fn unauthorized(reason: String) -> Self {
        OpRejection {
            kind: OpRejectionKind::PassNotAuthorized,
            reason,
        }
    }

    #[must_use]
    pub fn from_failure(failure: ComputerFailure) -> Self {
        let kind = match failure.kind {
            ComputerFailureKind::Unreachable => OpRejectionKind::Unreachable,
            ComputerFailureKind::Operation => OpRejectionKind::Operation,
        };
        OpRejection {
            kind,
            reason: failure.reason,
        }
    }
}

pub trait GatedComputer: Send + Sync {
    fn name(&self) -> &str;

    fn declared_caps(&self) -> ComputerCaps;

    fn capabilities(&self, pass: &Pass) -> Result<ActualResult, OpRejection>;

    fn run_command(
        &self,
        pass: &Pass,
        command: &str,
        timeout_ms: u64,
    ) -> Result<ActualResult, OpRejection>;

    fn read_file(&self, pass: &Pass, path: &str) -> Result<ActualResult, OpRejection>;

    fn write_file(
        &self,
        pass: &Pass,
        path: &str,
        content: &str,
    ) -> Result<ActualResult, OpRejection>;

    fn edit_file(
        &self,
        pass: &Pass,
        path: &str,
        old_text: &str,
        new_text: &str,
    ) -> Result<ActualResult, OpRejection>;

    fn path_info(&self, pass: &Pass, path: &str) -> Result<ActualResult, OpRejection>;

    fn resolve_target(&self, path: &str) -> Option<String>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_op_rejection_maps_computer_failure_kinds() {
        let unreachable =
            OpRejection::from_failure(ComputerFailure::unreachable("no route to host".to_owned()));
        let operation =
            OpRejection::from_failure(ComputerFailure::operation("disk full".to_owned()));
        assert_eq!(unreachable.kind, OpRejectionKind::Unreachable);
        assert_eq!(operation.kind, OpRejectionKind::Operation);
        assert_eq!(operation.reason, "disk full");
    }
}
