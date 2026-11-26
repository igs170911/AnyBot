fn peek(ticket: &anybot_trust::AskTicket) -> anybot_protocol::GateRecord {
    let anybot_trust::AskTicket { gate, .. } = ticket;
    gate.clone()
}

fn main() {
    let referenced = peek;
    let _ = referenced;
}
