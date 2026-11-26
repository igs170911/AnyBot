use anybot_protocol::GateOutcome;

fn main() {
    let verdict = anybot_trust::GateVerdict {
        outcome: GateOutcome::Allow,
        layer: anybot_protocol::GateLayer::Default,
        rule: "made-by-me".to_owned(),
        read_only: false,
        params: Vec::new(),
    };
    println!("{verdict:?}");
}
