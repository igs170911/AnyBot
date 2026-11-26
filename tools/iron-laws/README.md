# iron-laws passguard

Scans every `.rs` file outside `crates/trust/` for tokens that construct an
operation ticket: `OpTicket::issue`, `OpTicket {`, `OpTicket{`. Rust string
literals are stripped before scanning, so test fixtures that mention these
words are not flagged.

## What it checks

A textual scan of the whole repository for the three ticket-construction
tokens in any Rust source outside the trust crate. It runs in `make check`
(the iron-laws gate) and in the fast gate.

## What it misses

- Text split across lines, for example `OpTicket\n{` after formatting, is not
  matched by the two single-line tokens; struct literals written in one line
  are matched.
- Macro expansion, `build.rs` output, and generated files are not scanned.
- Renaming imports and wrapper functions (for example a helper in a third
  crate that calls `OpTicket::issue` inside `crates/trust/`) are not
  detected; the scan only sees the tokens in the file they appear in.
- Because `OpTicket::issue` is `pub(crate)` in the trust crate, code outside
  the crate cannot compile against it; the textual scan is the belt, the
  visibility is the suspenders.

## Compile-fail tests that close the gaps

`crates/trust/tests/compilefail/*.rs` (run through trybuild) prove the
compiler rejects each bypass the scanner can miss:

| Test | What it proves |
| --- | --- |
| `issue_ticket_outside_trust.rs` | `OpTicket::issue` is not visible outside `crates/trust/`. |
| `clone_ticket_outside_trust.rs` | `OpTicket` has no `Clone`, so a ticket cannot be copied. |
| `build_gate_verdict_outside_trust.rs` | `GateVerdict` is not public, so it cannot be built by hand. |
| `edit_gate_verdict_outside_trust.rs` | There is no public verdict to mutate, so "allow" cannot be forged. |
| `build_ask_ticket_outside_trust.rs` | `AskTicket` fields are private; it cannot be constructed. |
| `clone_ask_ticket_outside_trust.rs` | `AskTicket` has no `Clone`, so a ticket cannot be copied. |
| `build_evidence_outside_trust.rs` | The evidence builders are not visible outside `crates/trust/`. |
| `read_pass_field_outside_trust.rs` | `ActionRun` has no pass or ticket field, so a finished run hands the caller nothing to replay an operation with. |
| `run_command_without_ticket.rs` | Every computer operation takes exactly one `OpTicket`; a call without one does not compile. An implementation outside `crates/trust` can exist, but it can never obtain a ticket to invoke its own operations with, so it can only forward tickets built by the gate. |
