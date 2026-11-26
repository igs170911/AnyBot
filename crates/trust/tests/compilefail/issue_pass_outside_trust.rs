use anybot_protocol::{GateOutcome, GateNo, JudgementSource, RuleLayer};
use anybot_protocol::GateRecord;
use anybot_trust::{ActionKey, Pass};

fn main() {
    let key = ActionKey::command("shell", "ls");
    let record = GateRecord::new(
        GateNo(1),
        RuleLayer::Default,
        "test-rule".to_owned(),
        JudgementSource::Rule,
        GateOutcome::Allow,
        false,
    );
    let pass = Pass::issue(key, record, anybot_foundation::TimestampMs::from_millis(1));
    println!("{pass:?}");
}
