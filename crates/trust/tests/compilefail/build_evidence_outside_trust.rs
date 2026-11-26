fn pick(context: &anybot_trust::ActionContext, gate: &anybot_protocol::GateRecord) {
    let record = anybot_trust::evidence::build_evidence(
        context,
        gate,
        &anybot_protocol::ActualResult::Caps(anybot_protocol::ComputerCaps::declared(
            anybot_protocol::ComputerPlatform::Linux,
            anybot_protocol::ComputerOwner::Bot,
        )),
        &anybot_foundation::SystemClock,
    );
    println!("{record:?}");
}

fn main() {
    let referenced = pick;
    let _ = referenced;
}
