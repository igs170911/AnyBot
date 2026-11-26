extern crate regex as _;
extern crate spec_trace as _;

use std::process::Command;

#[test]
fn test_spec_trace_without_a_base_ref_fails() {
    let out = Command::new(env!("CARGO_BIN_EXE_spec-trace"))
        .args(["--against", ""])
        .output()
        .unwrap();
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(!out.status.success(), "output: {text}");
    assert!(text.contains("no comparison base"), "output: {text}");
}
