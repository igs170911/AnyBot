fn pick(input: &anybot_trust::gate_full::EngineInput) {
    println!("{input:?}");
}

fn main() {
    let referenced = pick;
    let _ = referenced;
}
