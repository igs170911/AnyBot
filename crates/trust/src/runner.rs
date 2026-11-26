use anybot_protocol::{ActualResult, ComputerCaps, ComputerFailure, ComputerFailureKind};

use crate::op_ticket::OpTicket;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpRejectionKind {
    TicketMismatch,
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
    pub fn mismatch(reason: String) -> Self {
        OpRejection {
            kind: OpRejectionKind::TicketMismatch,
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

    fn capabilities(&self, ticket: OpTicket) -> Result<ActualResult, OpRejection>;

    fn run_command(&self, ticket: OpTicket) -> Result<ActualResult, OpRejection>;

    fn read_file(&self, ticket: OpTicket) -> Result<ActualResult, OpRejection>;

    fn write_file(&self, ticket: OpTicket) -> Result<ActualResult, OpRejection>;

    fn edit_file(&self, ticket: OpTicket) -> Result<ActualResult, OpRejection>;

    fn path_info(&self, ticket: OpTicket) -> Result<ActualResult, OpRejection>;

    fn resolve_target(&self, path: &str) -> Option<String>;

    fn bound_path_still(&self, ticket: &OpTicket) -> bool {
        let Some(bound) = ticket.bound_path() else {
            return true;
        };
        self.resolve_target(bound).as_deref() == Some(bound)
    }

    fn declared_workdir(&self) -> String;

    fn gate_config(&self) -> crate::gate_flow::GateConfig {
        crate::gate_flow::GateConfig::plain()
    }
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
