#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingScope {
    Synced { per_item: bool },
    DeviceOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SettingSpec {
    pub name: &'static str,
    pub scope: SettingScope,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingCheck {
    Synced { per_item: bool },
    Unknown,
    DeviceOnly,
}

pub const SETTINGS: [SettingSpec; 9] = [
    SettingSpec {
        name: "accent_color",
        scope: SettingScope::Synced { per_item: false },
    },
    SettingSpec {
        name: "density",
        scope: SettingScope::Synced { per_item: false },
    },
    SettingSpec {
        name: "bot_name",
        scope: SettingScope::Synced { per_item: true },
    },
    SettingSpec {
        name: "pinned",
        scope: SettingScope::Synced { per_item: true },
    },
    SettingSpec {
        name: "theme",
        scope: SettingScope::DeviceOnly,
    },
    SettingSpec {
        name: "notification_sound",
        scope: SettingScope::DeviceOnly,
    },
    SettingSpec {
        name: "run_here",
        scope: SettingScope::DeviceOnly,
    },
    SettingSpec {
        name: "device_name",
        scope: SettingScope::DeviceOnly,
    },
    SettingSpec {
        name: "phone_control",
        scope: SettingScope::DeviceOnly,
    },
];

#[must_use]
pub fn check_setting(name: &str) -> SettingCheck {
    match SETTINGS.iter().find(|spec| spec.name == name) {
        None => SettingCheck::Unknown,
        Some(spec) => match spec.scope {
            SettingScope::Synced { per_item } => SettingCheck::Synced { per_item },
            SettingScope::DeviceOnly => SettingCheck::DeviceOnly,
        },
    }
}
