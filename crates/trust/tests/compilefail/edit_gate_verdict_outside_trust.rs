fn main() {
    let config = anybot_trust::GateConfig::plain();
    println!("{config:?}");
    let mut verdict = anybot_trust::gate_verdict_for_testing();
    verdict.outcome = anybot_protocol::GateOutcome::Allow;
}
