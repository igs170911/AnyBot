# anybot-computers

This crate is the computer interface only: the capability and result types
from `anybot-protocol` and the `GatedComputer` trait from `anybot-trust`.
There is no real computer endpoint in this milestone (the task says the real
Bot computer comes later).

## Interface rule

Every `GatedComputer` implementation must check, at the start of every
operation, that the `Pass` handed to it was issued for exactly that
operation (same tool and same action key). A pass for one operation must
never run another one: the implementation rejects the call before touching
the machine.

This is enforced in the fake computer used by tests (`tests/runner-test`,
`FakeComputer::authorize`), which records the rejection `pass does not
authorize ...` when the key does not match. The test
`test_the_computer_rejects_a_pass_issued_for_a_different_action` proves an
`ls` pass cannot run `pwd`.

A real endpoint must also:

- deny the always-denied command list (`sandbox-deny` S4) by itself,
  without relying on the gate,
- not claim a command sandbox it does not have.
