use crate::settings_registry::{SettingCheck, check_setting};

#[test]
fn synced_fields_report_whether_they_are_per_item() {
    assert_eq!(
        check_setting("accent_color"),
        SettingCheck::Synced { per_item: false }
    );
    assert_eq!(
        check_setting("bot_name"),
        SettingCheck::Synced { per_item: true }
    );
}

#[test]
fn device_only_fields_are_refused_as_device_only() {
    assert_eq!(check_setting("theme"), SettingCheck::DeviceOnly);
    assert_eq!(check_setting("phone_control"), SettingCheck::DeviceOnly);
}

#[test]
fn names_outside_the_registry_are_unknown() {
    assert_eq!(check_setting("accentcolour"), SettingCheck::Unknown);
    assert_eq!(check_setting(""), SettingCheck::Unknown);
}

#[test]
fn autonomy_switches_follow_the_account_across_devices() {
    assert_eq!(
        check_setting("yolo"),
        SettingCheck::Synced { per_item: false }
    );
    assert_eq!(
        check_setting("direct_mode"),
        SettingCheck::Synced { per_item: false }
    );
}
