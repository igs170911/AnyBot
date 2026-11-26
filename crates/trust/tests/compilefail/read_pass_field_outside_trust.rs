fn take(run: anybot_trust::ActionRun) {
    let handed = run.pass;
    println!("{handed:?}");
}

fn main() {
    let referenced = take;
    let _ = referenced;
}
