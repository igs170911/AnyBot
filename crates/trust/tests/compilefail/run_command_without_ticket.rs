use anybot_trust::GatedComputer;

fn pick(computer: &dyn GatedComputer) {
    let outcome = computer.run_command();
    println!("{outcome:?}");
}

fn main() {
    let referenced: fn(&dyn GatedComputer) = pick;
    let _ = referenced;
}
