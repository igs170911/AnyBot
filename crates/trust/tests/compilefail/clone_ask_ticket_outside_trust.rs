fn main() {
    let config = anybot_trust::GateConfig::plain();
    let ticket: Option<anybot_trust::AskTicket> = None;
    let copied = ticket.clone();
    println!("{config:?} {copied:?}");
}
