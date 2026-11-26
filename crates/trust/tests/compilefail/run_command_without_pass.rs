use anybot_trust::GatedComputer;

fn pick(computer: &dyn GatedComputer) {
    let outcome = computer.run_command("ls -la", 1000);
    println!("{outcome:?}");
}

fn main() {
    pick(&());
}
