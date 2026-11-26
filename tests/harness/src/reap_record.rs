use std::path::Path;

use super::{parse_record, path_under};

#[test]
fn reap_parse_record_rejects_blank_and_zero() {
    assert!(parse_record("").is_none());
    assert!(parse_record("0").is_none());
    let parsed = parse_record("42\n/work/m6a/target/debug/testenv\n/work/m6a\n").expect("record");
    assert_eq!(parsed.pid, 42);
    assert_eq!(
        parsed.exe.as_deref(),
        Some(Path::new("/work/m6a/target/debug/testenv"))
    );
    assert_eq!(parsed.workspace.as_deref(), Some(Path::new("/work/m6a")));
}

#[test]
fn reap_path_under_requires_the_workspace_prefix() {
    let root = Path::new("/work/m6a");
    assert!(path_under(
        Path::new("/work/m6a/target/debug/testenv"),
        root
    ));
    assert!(!path_under(
        Path::new("/work/m7a/target/debug/testenv"),
        root
    ));
}
