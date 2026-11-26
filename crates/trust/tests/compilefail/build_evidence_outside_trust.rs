use anybot_trust::ActionContext;

fn pick(context: &ActionContext, gate: &anybot_protocol::GateRecord) {
    let record = anybot_trust::build_evidence(context, gate, &anybot_protocol::ActualResult::Read(anybot_protocol::ReadOutcome {
        path: "/w/a".to_owned(),
        kind: anybot_protocol::TargetKind::File,
        digest: anybot_foundation::ContentDigest::sha256(b"a"),
        size: 1,
        content: "a".to_owned(),
    }), &anybot_foundation::SystemClock);
    println!("{record:?}");
}

fn main() {
    let gate = anybot_protocol::GateRecord::new(
        anybot_protocol::GateNo(1),
        anybot_protocol::RuleLayer::Default,
        "r".to_owned(),
        anybot_protocol::JudgementSource::Rule,
        anybot_protocol::GateOutcome::Allow,
        true,
    );
    pick(
        &ActionContext {
            task: anybot_foundation::TaskId::generate(&anybot_foundation::SystemClock),
            turn: anybot_foundation::TurnId::generate(&anybot_foundation::SystemClock),
            evidence_no: 1,
            computer: "c".to_owned(),
            workdir: "/w".to_owned(),
            tool: "shell".to_owned(),
            classes: Vec::new(),
        },
        &gate,
    );
}
