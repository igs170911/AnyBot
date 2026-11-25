use super::{is_slash_wrapped, matches_done_when};

#[test]
fn t_goal_followup_13() {
    assert!(matches_done_when("/\\d+%/", "progress 42% done"));
    assert!(!matches_done_when("/\\d+%/", "no percent here"));
    assert!(matches_done_when("DONE", "echo DONE at the end"));
    assert!(!matches_done_when("DONE", "echo nothing"));
    assert!(is_slash_wrapped("/ab/"));
    assert!(is_slash_wrapped("/a/"));
    assert!(!is_slash_wrapped("//"));
    assert!(!is_slash_wrapped("slash / mid /"));
}

#[test]
fn test_followup_broken_regex_falls_back_to_plain() {
    assert!(matches_done_when("/((/", "text /((/ tail"));
}
