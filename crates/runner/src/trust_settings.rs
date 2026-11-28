use anybot_foundation::BotId;
use anybot_trust::{ActionKey, AlwaysEntry, ComputerUse, LocalPolicy, ModeFlags, UserRules};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ConversationKey(pub u64);

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum TrustSettingsError {
    #[error("approval rule is unreadable: {rule}")]
    Unreadable { rule: String },
    #[error("trust settings are unavailable")]
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComputerUseChange {
    AllowBot,
    RunHereAlways,
}

pub trait TrustSettings: Send + Sync {
    fn computer_use(&self, bot: &BotId, computer: &str) -> Result<ComputerUse, TrustSettingsError>;

    fn policy(&self, computer: &str) -> Result<LocalPolicy, TrustSettingsError>;

    fn modes(&self) -> Result<ModeFlags, TrustSettingsError>;

    fn user_rules(
        &self,
        bot: &BotId,
        conversation: ConversationKey,
    ) -> Result<UserRules, TrustSettingsError>;

    fn allow_in_conversation(
        &self,
        bot: &BotId,
        conversation: ConversationKey,
        key: &ActionKey,
    ) -> Result<(), TrustSettingsError>;

    fn add_always(&self, entry: AlwaysEntry) -> Result<(), TrustSettingsError>;

    fn remove_ask_rule(&self, rule: &str) -> Result<(), TrustSettingsError>;

    fn set_computer_use(
        &self,
        bot: &BotId,
        computer: &str,
        change: ComputerUseChange,
    ) -> Result<(), TrustSettingsError>;

    fn set_direct_mode(&self, on: bool) -> Result<(), TrustSettingsError>;
}
