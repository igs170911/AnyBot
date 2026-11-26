fn seal(verdict: &mut anybot_trust::GateVerdict) {
    verdict.rule = "made-by-me".to_owned();
}

fn main() {
    let referenced = seal;
    let _ = referenced;
}
