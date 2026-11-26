fn copy(ticket: anybot_trust::OpTicket) -> anybot_trust::OpTicket {
    ticket.clone()
}

fn main() {
    let referenced = copy;
    let _ = referenced;
}
