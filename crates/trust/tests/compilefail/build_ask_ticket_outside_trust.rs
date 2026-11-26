fn main() {
    let ticket = anybot_trust::AskTicket {
        gate: anybot_protocol::GateRecord::default(),
        card: anybot_trust::CardData::default(),
    };
    println!("{ticket:?}");
}
