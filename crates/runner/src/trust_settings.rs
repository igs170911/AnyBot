use anybot_foundation::BotId;
use anybot_trust::{ComputerUse, LocalPolicy, ModeFlags, UserRules};
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

pub trait TrustSettings: Send + Sync {
    fn computer_use(&self, bot: &BotId, computer: &str) -> Result<ComputerUse, TrustSettingsError>;

    fn policy(&self, computer: &str) -> Result<LocalPolicy, TrustSettingsError>;

    fn modes(&self) -> Result<ModeFlags, TrustSettingsError>;

    fn user_rules(
        &self,
        bot: &BotId,
        conversation: ConversationKey,
    ) -> Result<UserRules, TrustSettingsError>;
}
