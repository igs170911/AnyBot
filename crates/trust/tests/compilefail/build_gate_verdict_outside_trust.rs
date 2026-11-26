use anybot_protocol::GateOutcome;
use anybot_trust::GateLayer;

fn main() {
    let verdict = anybot_trust::GateVerdict {
        outcome: GateOutcome::Allow,
        layer: GateLayer::Default,
        rule: "made-by-me".to_owned(),
        read_only: false,
        params: Vec::new(),
    };
    println!("{verdict:?}");
}
