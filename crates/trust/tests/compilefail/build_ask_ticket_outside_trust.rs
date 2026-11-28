fn main() {
    let ticket = anybot_trust::AskTicket::new(
        anybot_protocol::GateOutcome::Ask,
        anybot_protocol::GateRecord::new(
            anybot_protocol::GateNo(1),
            anybot_protocol::RuleLayer::Default,
            "r".to_owned(),
            anybot_protocol::JudgementSource::Rule,
            anybot_protocol::GateOutcome::Ask,
            false,
        ),
        anybot_trust::GateAction::command("shell", "ls", "/workspace", Vec::new()),
        &anybot_protocol::ComputerCaps::declared(
            anybot_protocol::ComputerPlatform::Linux,
            anybot_protocol::ComputerOwner::Bot,
        ),
        &anybot_trust::ActionRequest {
            context: anybot_trust::ActionContext {
                task: anybot_foundation::TaskId::generate(&anybot_foundation::SystemClock),
                turn: anybot_foundation::TurnId::generate(&anybot_foundation::SystemClock),
                evidence_no: 1,
                computer: "c".to_owned(),
                workdir: "/workspace".to_owned(),
                tool: "shell".to_owned(),
                classes: Vec::new(),
            },
            subject: anybot_trust::RequestedAction::Command {
                line: "ls".to_owned(),
                timeout_ms: 1000,
                env: anybot_trust::command_env_plan(),
            },
        },
        "c",
        &anybot_trust::CallContext::new(
            anybot_foundation::CallId::generate(&anybot_foundation::SystemClock),
            anybot_foundation::TurnId::generate(&anybot_foundation::SystemClock),
            "c1",
        ),
    );
    println!("{}", ticket.computer_name());
}
