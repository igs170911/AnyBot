use anybot_protocol::{GateOutcome, GateStep, JudgementSource, RuleLayer};

fn main() {
    let subject = anybot_trust::ActionSubject::Capabilities;
    let key = anybot_trust::ActionKey::capabilities("shell");
    let gate = anybot_protocol::GateRecord::new(
        GateStep::DefaultRule,
        RuleLayer::Default,
        "r".to_owned(),
        JudgementSource::Rule,
        GateOutcome::Allow,
        false,
    );
    let ticket = anybot_trust::OpTicket::issue(
        &subject,
        key,
        gate,
        "c",
        &anybot_protocol::ComputerCaps::declared(
            anybot_protocol::ComputerPlatform::Linux,
            anybot_protocol::ComputerOwner::Bot,
        ),
        "/workspace",
        anybot_foundation::TimestampMs::from_millis(1),
    );
    println!("{ticket:?}");
}
