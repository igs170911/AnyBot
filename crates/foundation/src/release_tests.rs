use super::{
    DEV_APP_IDENTIFIER, DEV_APP_NAME, DEV_CHANNEL, STABLE_APP_IDENTIFIER, STABLE_APP_NAME,
    STABLE_CHANNEL, version_line,
};

#[test]
fn version_line_uses_short_hash_and_rfc3339() {
    let line = version_line("0.1.0", "v0.1.0-3-gabc1234", Some("1732523400"));
    assert_eq!(line, "v0.1.0-abc1234 · 2024-11-25T08:30:00Z");
}

#[test]
fn version_line_keeps_dirty_marker() {
    let line = version_line("0.1.0", "abc1234-dirty", Some("1732523400"));
    assert!(line.starts_with("v0.1.0-abc1234-dirty · "));
}

#[test]
fn version_line_without_epoch_is_stable() {
    let line = version_line("0.1.0", "abc1234", None);
    assert_eq!(line, "v0.1.0-abc1234 · unspecified");
}

#[test]
fn release_channel_names_do_not_share_a_data_dir() {
    assert_ne!(STABLE_APP_IDENTIFIER, DEV_APP_IDENTIFIER);
    assert_ne!(STABLE_APP_NAME, DEV_APP_NAME);
    assert_eq!(STABLE_APP_IDENTIFIER, "app.anybot.desktop");
    assert_eq!(STABLE_CHANNEL, "stable");
    assert_eq!(DEV_CHANNEL, "dev");
    assert_eq!(DEV_APP_NAME, "AnyBot Dev");
}

#[test]
fn log_version_strips_one_leading_v() {
    assert_eq!(
        super::log_version("v0.1.0-abc · 2024-11-25T08:30:00Z"),
        "0.1.0-abc · 2024-11-25T08:30:00Z"
    );
    assert_eq!(super::log_version("0.1.0"), "0.1.0");
}

#[test]
fn default_compile_is_the_stable_channel() {
    assert_eq!(super::release_channel(), STABLE_CHANNEL);
    assert_eq!(super::app_identifier(), STABLE_APP_IDENTIFIER);
    assert_eq!(super::app_name(), STABLE_APP_NAME);
    assert!(!super::release_is_dev());
}
