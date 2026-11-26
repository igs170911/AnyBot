use crate::sandbox_peel::{
    CODE_TOOL_EVAL, CODE_TOOL_HEAD, CODE_TOOL_HEREDOC, CODE_TOOL_UNBALANCED, bare_name,
    peel_segment, unreadable_reason, visible_segments,
};

#[test]
fn test_peeling_stops_only_when_the_head_is_not_a_wrapper() {
    assert_eq!(
        peel_segment("sudo nohup mkfs.ext4 /dev/sda1", true),
        "mkfs.ext4 /dev/sda1"
    );
    assert_eq!(peel_segment("sudo env FOO=1 reboot", true), "reboot");
    assert_eq!(peel_segment("env PATH=/bin rm -rf ~", false), "rm -rf ~");
    assert_eq!(peel_segment("sudo ls -la", false), "ls -la");
}

#[test]
fn test_bare_name_reads_the_last_piece_of_a_path_and_drops_quotes() {
    assert_eq!(bare_name("/sbin/reboot"), "reboot");
    assert_eq!(bare_name("'reboot'"), "reboot");
    assert_eq!(bare_name("reboot"), "reboot");
}

#[test]
fn test_visible_segments_keep_only_the_small_recognized_set() {
    let got = visible_segments("echo hi ; ls -la");
    assert_eq!(got, vec!["echo hi".to_owned(), "ls -la".to_owned()]);
    assert!(visible_segments("( mkfs.ext4 /dev/sda1 )").is_empty());
    assert!(visible_segments("ls $(mkfs.ext4 /dev/sda1)").is_empty());
    assert!(visible_segments("sh -c 'ls && mkfs.ext4 /dev/sda1'").is_empty());
}

#[test]
fn test_unreadable_commands_carry_their_own_reason_code() {
    assert_eq!(unreadable_reason("eval ls"), Some(CODE_TOOL_EVAL));
    assert_eq!(unreadable_reason("echo \"abc"), Some(CODE_TOOL_UNBALANCED));
    assert_eq!(unreadable_reason("bash << EOF"), Some(CODE_TOOL_HEREDOC));
    assert_eq!(unreadable_reason("$X ls"), Some(CODE_TOOL_HEAD));
    assert_eq!(unreadable_reason("ls -la"), None);
}
