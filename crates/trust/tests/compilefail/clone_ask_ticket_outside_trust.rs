fn copy(ticket: anybot_trust::AskTicket) -> anybot_trust::AskTicket {
    ticket.clone()
}

fn main() {
    let referenced = copy;
    let _ = referenced;
}
