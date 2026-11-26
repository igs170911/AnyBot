use crate::sandbox_peel::{
    CODE_TOOL_EVAL, CODE_TOOL_HEREDOC, CODE_TOOL_UNBALANCED, bare_name, peel_segment,
    unreadable_reason, visible_segments,
};

#[test]
fn test_peeling_stops_only_when_the_head_is_not_a_wrapper() {
    assert_eq!(
        peel_segment("sudo nohup mkfs.ext4 /dev/sda1", true),
        "mkfs.ext4 /dev/sda1"
    );
    assert_eq!(peel_segment("sudo env FOO=1 reboot", true), "reboot");
    assert_eq!(peel_segment("env PATH=/bin rm -rf ~", false), "rm -rf ~");
    assert_eq!(peel_segment("sudo ls -la", false), "sudo ls -la");
}

#[test]
fn test_bare_name_reads_the_last_piece_of_a_path_and_drops_quotes() {
    assert_eq!(bare_name("/sbin/reboot"), "reboot");
    assert_eq!(bare_name("'reboot'"), "reboot");
    assert_eq!(bare_name("reboot"), "reboot");
}

#[test]
fn test_visible_segments_split_compound_keywords_and_read_subshells() {
    let got = visible_segments("if true; then mkfs.ext4 /dev/sda1; fi");
    assert!(got.iter().any(|s| s.starts_with("mkfs.ext4 /dev/sda1")));
    let sub = visible_segments("ls $(mkfs.ext4 /dev/sda1)");
    assert!(sub.iter().any(|s| s.starts_with("mkfs.ext4 /dev/sda1")));
    let tick = visible_segments("ls `mkfs.ext4 /dev/sda1`");
    assert!(tick.iter().any(|s| s.starts_with("mkfs.ext4 /dev/sda1")));
    let paren = visible_segments("bash ( mkfs.ext4 /dev/sda1 )");
    assert!(paren.iter().any(|s| s.starts_with("mkfs.ext4 /dev/sda1")));
}

#[test]
fn test_visible_segments_split_the_script_inside_a_shell_quote() {
    let got = visible_segments("sh -c 'ls && mkfs.ext4 /dev/sda1'");
    assert!(got.iter().any(|s| s.starts_with("mkfs.ext4 /dev/sda1")));
    let bash = visible_segments("bash -c \"dd of=/x\"");
    assert!(bash.iter().any(|s| s.starts_with("dd")));
}

#[test]
fn test_unreadable_commands_carry_their_own_reason_code() {
    assert_eq!(unreadable_reason("eval ls"), Some(CODE_TOOL_EVAL));
    assert_eq!(unreadable_reason("echo \"abc"), Some(CODE_TOOL_UNBALANCED));
    assert_eq!(unreadable_reason("bash << EOF"), Some(CODE_TOOL_HEREDOC));
    assert_eq!(unreadable_reason("$X ls"), Some("tool-unreadable-head"));
    assert_eq!(unreadable_reason("ls -la"), None);
}
