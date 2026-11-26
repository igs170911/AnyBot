fn pick(gate: &anybot_protocol::GateRecord) {
    let run = anybot_trust::finish(gate.clone(), Vec::new(), anybot_trust::OutcomeText::of("x", Vec::new()), &anybot_trust::ActionContext {
        task: anybot_foundation::TaskId::generate(&anybot_foundation::SystemClock),
        turn: anybot_foundation::TurnId::generate(&anybot_foundation::SystemClock),
        evidence_no: 1,
        computer: "c".to_owned(),
        workdir: "/w".to_owned(),
        tool: "shell".to_owned(),
        classes: Vec::new(),
    });
    println!("{run:?}");
}

fn main() {
    let referenced = pick;
    let _ = referenced;
}
