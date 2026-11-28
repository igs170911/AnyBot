use crate::action::{ActionKey, ActionSubject};
use crate::op_ticket::{OpKind, OpTicket};
use crate::runner::{GatedComputer, OpRejection};
use anybot_foundation::TimestampMs;
use anybot_protocol::{
    ActualResult, ComputerCaps, ComputerOwner, ComputerPlatform, GateNo, GateOutcome, GateRecord,
    JudgementSource, RuleLayer,
};

struct MinComputer;

fn ticket_caps() -> ComputerCaps {
    ComputerCaps::declared(ComputerPlatform::Linux, ComputerOwner::Bot)
}

fn allow_record() -> GateRecord {
    GateRecord::new(
        GateNo(1),
        RuleLayer::Default,
        "bot-machine-command".to_owned(),
        JudgementSource::Rule,
        GateOutcome::Allow,
        false,
    )
}

fn command_ticket(line: &str, computer: &str) -> OpTicket {
    let subject = ActionSubject::Command {
        line: line.to_owned(),
        timeout_ms: 1000,
    };
    let key = ActionKey::command("shell", line);
    OpTicket::issue(
        &subject,
        key,
        allow_record(),
        computer,
        &ticket_caps(),
        "/workspace",
        TimestampMs::from_millis(10),
    )
}

impl MinComputer {
    fn refuse(ticket: &OpTicket, kind: OpKind) -> Result<ActualResult, OpRejection> {
        if ticket.fits("min-computer", kind)
            && ticket.matches_computer("min-computer", &ticket_caps(), "/workspace")
        {
            return Ok(ActualResult::Caps(ticket_caps()));
        }
        Err(OpRejection::mismatch(format!(
            "the ticket names {} and a {:?} operation",
            ticket.computer(),
            ticket.kind(),
        )))
    }
}

impl GatedComputer for MinComputer {
    fn name(&self) -> &'static str {
        "min-computer"
    }

    fn declared_caps(&self) -> ComputerCaps {
        ticket_caps()
    }

    fn declared_workdir(&self) -> String {
        "/workspace".to_owned()
    }

    fn resolve_target(&self, _path: &str) -> Option<String> {
        None
    }

    fn capabilities(&self, ticket: OpTicket) -> Result<ActualResult, OpRejection> {
        Self::refuse(&ticket, OpKind::Caps)
    }

    fn run_command(&self, ticket: OpTicket) -> Result<ActualResult, OpRejection> {
        Self::refuse(&ticket, OpKind::Run)
    }

    fn read_file(&self, ticket: OpTicket) -> Result<ActualResult, OpRejection> {
        Self::refuse(&ticket, OpKind::Read)
    }

    fn list_dir(&self, ticket: OpTicket) -> Result<ActualResult, OpRejection> {
        Self::refuse(&ticket, OpKind::ListDir)
    }

    fn write_file(&self, ticket: OpTicket) -> Result<ActualResult, OpRejection> {
        Self::refuse(&ticket, OpKind::Write)
    }

    fn edit_file(&self, ticket: OpTicket) -> Result<ActualResult, OpRejection> {
        Self::refuse(&ticket, OpKind::Edit)
    }

    fn path_info(&self, ticket: OpTicket) -> Result<ActualResult, OpRejection> {
        Self::refuse(&ticket, OpKind::Path)
    }
}

#[test]
fn test_a_ticket_handed_to_another_computer_is_refused() {
    let ticket = command_ticket("ls -la", "other-computer");
    let refused = MinComputer.run_command(ticket);
    let rejection = refused.unwrap_err();
    assert_eq!(
        rejection.kind,
        crate::runner::OpRejectionKind::TicketMismatch
    );
    assert!(rejection.reason.contains("other-computer"));
}

#[test]
fn test_a_ticket_for_the_right_computer_and_the_right_operation_is_accepted() {
    let ticket = command_ticket("ls -la", "min-computer");
    assert!(MinComputer.run_command(ticket).is_ok());
}

struct RootWrapper;

impl GatedComputer for RootWrapper {
    fn name(&self) -> &'static str {
        "min-computer"
    }

    fn declared_caps(&self) -> ComputerCaps {
        ComputerCaps::declared(ComputerPlatform::BotMachine, ComputerOwner::Bot)
    }

    fn declared_workdir(&self) -> String {
        "/".to_owned()
    }

    fn resolve_target(&self, _path: &str) -> Option<String> {
        None
    }

    fn capabilities(&self, ticket: OpTicket) -> Result<ActualResult, OpRejection> {
        Self::spend(&ticket, OpKind::Caps)
    }

    fn run_command(&self, ticket: OpTicket) -> Result<ActualResult, OpRejection> {
        Self::spend(&ticket, OpKind::Run)
    }

    fn read_file(&self, ticket: OpTicket) -> Result<ActualResult, OpRejection> {
        Self::spend(&ticket, OpKind::Read)
    }

    fn list_dir(&self, ticket: OpTicket) -> Result<ActualResult, OpRejection> {
        Self::spend(&ticket, OpKind::ListDir)
    }

    fn write_file(&self, ticket: OpTicket) -> Result<ActualResult, OpRejection> {
        Self::spend(&ticket, OpKind::Write)
    }

    fn edit_file(&self, ticket: OpTicket) -> Result<ActualResult, OpRejection> {
        Self::spend(&ticket, OpKind::Edit)
    }

    fn path_info(&self, ticket: OpTicket) -> Result<ActualResult, OpRejection> {
        Self::spend(&ticket, OpKind::Path)
    }
}

impl RootWrapper {
    fn spend(ticket: &OpTicket, kind: OpKind) -> Result<ActualResult, OpRejection> {
        if !ticket.fits("min-computer", kind)
            || !ticket.matches_computer("min-computer", &ticket_caps(), "/")
        {
            return Err(OpRejection::mismatch(
                "the ticket was issued for another declaration".to_owned(),
            ));
        }
        Ok(ActualResult::Caps(ticket_caps()))
    }
}

#[test]
fn test_a_wrapper_with_the_real_name_and_a_root_workdir_cannot_spend_the_ticket() {
    let ticket = command_ticket("ls -la", "min-computer");
    let refused = RootWrapper::spend(&ticket, OpKind::Run);
    assert_eq!(
        refused.unwrap_err().kind,
        crate::runner::OpRejectionKind::TicketMismatch
    );
}
