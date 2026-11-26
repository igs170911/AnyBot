# iron-laws passguard

Scans every `.rs` file outside `crates/trust/` for tokens that construct a
`Pass`: `Pass::issue`, `Pass {`, `Pass{`. Rust string literals are stripped
before scanning, so test fixtures that mention these words are not flagged.

## What it checks

A textual scan of the whole repository for the three pass-construction tokens
in any Rust source outside the trust crate. It runs in `make check` (the
iron-laws gate) and in the fast gate.

## What it misses

- Text split across lines, for example `Pass\n{` after formatting, is not
  matched by the two single-line tokens; struct literals written in one line
  are matched.
- Macro expansion, `build.rs` output, and generated files are not scanned.
- Renaming imports and wrapper functions (for example a helper in a
  third crate that calls `Pass::issue` inside `crates/trust/`) are not
  detected; the scan only sees the tokens in the file they appear in.
- Because `Pass::issue` is `pub(crate)` in the trust crate, code outside the
  crate cannot compile against it; the textual scan is the belt, the
  visibility is the suspenders.

## Compile-fail tests that close the gaps

`crates/trust/tests/compilefail/*.rs` (run through trybuild) prove the
compiler rejects each bypass the scanner can miss:

| Test | What it proves |
| --- | --- |
| `issue_pass_outside_trust.rs` | `Pass::issue` is not visible outside `crates/trust/`. |
| `build_gate_verdict_outside_trust.rs` | `GateVerdict` is not public, so it cannot be built by hand. |
| `edit_gate_verdict_outside_trust.rs` | There is no public verdict to mutate, so "allow" cannot be forged. |
| `build_ask_ticket_outside_trust.rs` | `AskTicket` fields are private; it cannot be constructed. |
| `clone_ask_ticket_outside_trust.rs` | `AskTicket` has no `Clone`, so a ticket cannot be copied. |
| `clone_pass_outside_trust.rs` | `Pass` has no `Clone`, and no public API returns a `Pass`, so a pass cannot be copied. |
| `run_command_without_pass.rs` | The computer trait requires a pass for every operation; the trait is sealed, so it cannot be implemented outside. |
