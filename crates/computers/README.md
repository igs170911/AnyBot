# anybot-computers

This crate is the computer interface only: the capability and result types
from `anybot-protocol` and the `GatedComputer` trait from `anybot-trust`.
There is no real computer endpoint in this milestone (the task says the real
Bot computer comes later).

## Interface rule

Every operation on `GatedComputer` takes exactly one `OpTicket`, by value.
The ticket is the operation: it names the computer, the operation kind, and
every parameter in its original form (the command line, the path, the
content, the old and new text). There are no extra path, content, command,
or timeout parameters beside the ticket, so nothing can be passed that the
gate did not approve.

Only `crates/trust` can build a ticket (its fields are private, `OpTicket::issue`
is `pub(crate)`, and it has no `Clone` and no serde). Handing a ticket to an
operation consumes it: the call takes it by value, so a ticket can be used
once and cannot be stored, copied, or replayed.

An implementation must reject a ticket whose named computer is not itself,
or whose kind is not the operation being called, before touching the
machine. `FakeComputer::accept` in `tests/runner-test` does this and returns
`OpRejectionKind::TicketMismatch`; the trust-internal tests
`test_a_ticket_handed_to_another_computer_is_refused` and
`test_a_ticket_for_the_right_computer_and_the_right_operation_is_accepted`
prove both directions.

Nothing returned to a caller carries a ticket or a pass: `ActionRun` holds
only the gate record, the evidence, the model text, and the summary.

A real endpoint must also:

- deny the always-denied command list (`sandbox-deny` S4) by itself,
  without relying on the gate,
- not claim a command sandbox it does not have.
