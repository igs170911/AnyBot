use crate::safety_n5::*;

#[test]
fn test_egress_rules_split_segments_and_strip_wrappers() {
    let joined = classes_of_command("cd repo && git push origin");
    assert!(joined.contains(&ActionClass::Publish));
    let global = classes_of_command("git -C repo push origin");
    assert!(global.contains(&ActionClass::Publish));
    let wrapped = classes_of_command("nohup npm publish");
    assert!(wrapped.contains(&ActionClass::Publish));
    let newline = classes_of_command("ls\nsudo npm publish");
    assert!(newline.contains(&ActionClass::Publish));
    let plain = classes_of_command("cd repo && git status");
    assert!(!plain.contains(&ActionClass::Publish));
}

#[test]
fn t_safety_n5_1() {
    let delete_hits = [
        classes_of_command("rm -rf build"),
        classes_of_command("git rm notes.txt"),
    ];
    assert!(delete_hits[0].contains(&ActionClass::Delete));
    assert!(delete_hits[1].contains(&ActionClass::Delete));
    assert!(classes_of_connector_tool("linear_delete_issue", false).contains(&ActionClass::Delete));
    assert!(classes_of_click_label("Delete").contains(&ActionClass::Delete));
    let send_hits = [
        classes_of_connector_tool("slack_send_message", false),
        classes_of_command("mail bob@example.com"),
    ];
    assert!(send_hits[0].contains(&ActionClass::SendToOthers));
    assert!(send_hits[1].contains(&ActionClass::SendToOthers));
    let share_hits = [
        classes_of_connector_tool("gdrive_share_file", false),
        classes_of_click_label("Share"),
    ];
    assert!(share_hits[0].contains(&ActionClass::Share));
    assert!(share_hits[1].contains(&ActionClass::Share));
    let publish_hits = [
        classes_of_command("git push origin main"),
        classes_of_connector_tool("twitter_post", false),
    ];
    assert!(publish_hits[0].contains(&ActionClass::Publish));
    assert!(publish_hits[1].contains(&ActionClass::Publish));
    let pay_hits = [
        classes_of_connector_tool("stripe_checkout", false),
        classes_of_click_label("Checkout"),
    ];
    assert!(pay_hits[0].contains(&ActionClass::Payment));
    assert!(pay_hits[1].contains(&ActionClass::Payment));
    let pass_hits = [
        classes_of_connector_tool("vault_get_password", false),
        classes_of_command("passwd root"),
    ];
    assert!(pass_hits[0].contains(&ActionClass::PasswordChange));
    assert!(pass_hits[1].contains(&ActionClass::PasswordChange));
}

#[test]
fn test_six_class_negative_examples_stay_unclassified() {
    assert_eq!(classes_of_connector_tool("stripe_paypal", false), vec![]);
    assert_eq!(classes_of_click_label("reply"), vec![]);
    assert_eq!(classes_of_command("git pull origin main"), vec![]);
    assert_eq!(classes_of_command("echo \"rm old\""), vec![]);
    assert_eq!(classes_of_connector_tool("github_sender", false), vec![]);
    assert_eq!(classes_of_click_label("Search"), vec![]);
}

#[test]
fn test_git_global_options_with_equals_still_reach_the_subcommand() {
    let got = classes_of_command("git --git-dir=.git push origin");
    assert!(got.contains(&ActionClass::Publish));
}

#[test]
fn test_unknown_git_global_options_still_reach_the_subcommand() {
    let got = classes_of_command("git -P push origin");
    assert!(got.contains(&ActionClass::Publish));
}

#[test]
fn test_a_privilege_wrapper_hides_nothing_from_the_class_scan_on_the_bot_machine() {
    let got = classes_of_command("sudo nohup git push origin");
    assert!(got.contains(&ActionClass::Publish));
}

#[test]
fn test_a_class_scan_reads_the_command_inside_a_shell_quote() {
    let got = classes_of_command("bash -c 'git push origin'");
    assert!(got.contains(&ActionClass::Publish));
}
