fn pick(input: &anybot_trust::EngineInput) {
    let decided = anybot_trust::decide(input);
    println!("{decided:?}");
}

fn main() {
    let referenced = pick;
    let _ = referenced;
}
