use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapState {
    Supported,
    Unsupported,
}

impl CapState {
    #[must_use]
    pub fn yes(self) -> bool {
        self == CapState::Supported
    }

    #[must_use]
    pub fn word(self) -> &'static str {
        match self {
            CapState::Supported => "supported",
            CapState::Unsupported => "unsupported",
        }
    }

    #[must_use]
    pub fn of(yes: bool) -> Self {
        if yes {
            CapState::Supported
        } else {
            CapState::Unsupported
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum ComputerPlatform {
    BotMachine,
    Mac,
    Linux,
    Windows,
    Ssh,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComputerOwner {
    Bot,
    User,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComputerCaps {
    pub platform: ComputerPlatform,
    pub owner: ComputerOwner,
    pub command_sandbox: CapState,
    pub run_commands: CapState,
    pub read_files: CapState,
    pub write_files: CapState,
    pub check_path: CapState,
    pub screenshot: CapState,
    pub browser: CapState,
    pub desktop: CapState,
    pub screen_control: CapState,
    pub live_view: CapState,
    pub handover: CapState,
    pub sleep: CapState,
    pub restore: CapState,
    pub inject_secrets: CapState,
    pub open_for_user: CapState,
    pub receive_files: CapState,
    #[serde(default)]
    pub home_dir: String,
    #[serde(default)]
    pub anybot_dir: String,
    #[serde(default)]
    pub vault_paths: Vec<String>,
    #[serde(default)]
    pub keychain_dirs: Vec<String>,
    #[serde(default)]
    pub onepassword_dirs: Vec<String>,
}

impl ComputerCaps {
    #[must_use]
    pub fn declared(platform: ComputerPlatform, owner: ComputerOwner) -> Self {
        ComputerCaps {
            platform,
            owner,
            command_sandbox: CapState::Unsupported,
            run_commands: CapState::Unsupported,
            read_files: CapState::Unsupported,
            write_files: CapState::Unsupported,
            check_path: CapState::Unsupported,
            screenshot: CapState::Unsupported,
            browser: CapState::Unsupported,
            desktop: CapState::Unsupported,
            screen_control: CapState::Unsupported,
            live_view: CapState::Unsupported,
            handover: CapState::Unsupported,
            sleep: CapState::Unsupported,
            restore: CapState::Unsupported,
            inject_secrets: CapState::Unsupported,
            open_for_user: CapState::Unsupported,
            receive_files: CapState::Unsupported,
            home_dir: String::new(),
            anybot_dir: String::new(),
            vault_paths: vec!["vault.bin".to_owned(), "vault.key".to_owned()],
            keychain_dirs: platform_keychain_dirs(platform),
            onepassword_dirs: platform_onepassword_dirs(platform),
        }
    }

    #[must_use]
    pub fn declared_private_paths(&self) -> Vec<String> {
        let mut all = Vec::new();
        all.extend(self.vault_paths.iter().cloned());
        all.extend(self.keychain_dirs.iter().cloned());
        all.extend(self.onepassword_dirs.iter().cloned());
        all
    }

    #[must_use]
    pub fn capability_label(&self) -> String {
        format!(
            "sandbox {} commands {} read {} write {} path {} screenshots {} screen-control {} browser {}",
            self.command_sandbox.word(),
            self.run_commands.word(),
            self.read_files.word(),
            self.write_files.word(),
            self.check_path.word(),
            self.screenshot.word(),
            self.screen_control.word(),
            self.browser.word()
        )
    }

    #[must_use]
    pub fn bot_machine_can_operate_files(&self) -> bool {
        self.owner == ComputerOwner::Bot
            && self.read_files == CapState::Supported
            && self.write_files == CapState::Supported
    }
}

fn platform_keychain_dirs(platform: ComputerPlatform) -> Vec<String> {
    match platform {
        ComputerPlatform::Mac => vec![
            "Library/Keychains".to_owned(),
            "/Library/Keychains".to_owned(),
            "/System/Library/Keychains".to_owned(),
        ],
        ComputerPlatform::Linux => vec![".local/share/keyrings".to_owned()],
        ComputerPlatform::Windows => vec![
            "AppData/Local/Microsoft/Credentials".to_owned(),
            "AppData/Roaming/Microsoft/Credentials".to_owned(),
            "AppData/Local/Microsoft/Vault".to_owned(),
        ],
        ComputerPlatform::BotMachine | ComputerPlatform::Ssh | ComputerPlatform::Unknown => vec![
            "Library/Keychains".to_owned(),
            ".local/share/keyrings".to_owned(),
        ],
    }
}

fn platform_onepassword_dirs(platform: ComputerPlatform) -> Vec<String> {
    match platform {
        ComputerPlatform::Mac => vec![
            "Library/Group Containers/2BUA8C4S2C.com.1password".to_owned(),
            "Library/Application Support/1Password".to_owned(),
            ".1password".to_owned(),
        ],
        ComputerPlatform::Linux => vec![".config/1Password".to_owned(), ".1password".to_owned()],
        ComputerPlatform::Windows => {
            vec![
                "AppData/Local/1Password".to_owned(),
                ".1password".to_owned(),
            ]
        }
        ComputerPlatform::BotMachine | ComputerPlatform::Ssh | ComputerPlatform::Unknown => vec![
            ".config/1Password".to_owned(),
            ".1password".to_owned(),
            "Library/Group Containers/2BUA8C4S2C.com.1password".to_owned(),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_computer_caps_undeclared_capabilities_default_to_unsupported() {
        let caps = ComputerCaps::declared(ComputerPlatform::Linux, ComputerOwner::User);
        assert_eq!(caps.command_sandbox, CapState::Unsupported);
        assert_eq!(caps.screenshot, CapState::Unsupported);
        assert!(!caps.command_sandbox.yes());
    }

    #[test]
    fn test_computer_caps_named_overrides_apply_to_declared_fields() {
        let caps = ComputerCaps {
            run_commands: CapState::Supported,
            ..ComputerCaps::declared(ComputerPlatform::Windows, ComputerOwner::User)
        };
        assert_eq!(caps.run_commands, CapState::Supported);
        assert_eq!(caps.browser, CapState::Unsupported);
    }
}
