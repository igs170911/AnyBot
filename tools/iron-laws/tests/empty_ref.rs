extern crate iron_laws as _;
extern crate serde_json as _;
extern crate sqlparser as _;
extern crate yaml_serde as _;

use std::process::Command;

#[test]
fn test_iron_laws_without_a_base_ref_fails() {
    let out = Command::new(env!("CARGO_BIN_EXE_iron-laws"))
        .arg("")
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
