fn pick(input: &anybot_trust::EngineInput) {
    println!("{input:?}");
}

fn main() {
    let referenced = pick;
    let _ = referenced;
}
