use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use anybot_foundation::BotId;
use anybot_runner::{ConversationKey, TrustSettings, TrustSettingsError};
use anybot_trust::{ComputerUse, LocalPolicy, ModeFlags, RunHere, UserRules};

use crate::fake_computer::FakeComputer;

pub struct FakeTrust {
    pub(crate) computer: Arc<FakeComputer>,
    pub(crate) uses: Mutex<BTreeMap<String, ComputerUse>>,
    pub(crate) policies: Mutex<BTreeMap<String, LocalPolicy>>,
    pub(crate) conversation_allowed: Mutex<BTreeMap<ConversationKey, Vec<anybot_trust::ActionKey>>>,
}

impl FakeTrust {
    #[must_use]
    pub fn new(computer: Arc<FakeComputer>) -> Self {
        FakeTrust {
            computer,
            uses: Mutex::new(BTreeMap::new()),
            policies: Mutex::new(BTreeMap::new()),
            conversation_allowed: Mutex::new(BTreeMap::new()),
        }
    }

    #[must_use]
    pub fn default_use() -> ComputerUse {
        ComputerUse {
            usable_by_bot: true,
            run_here: RunHere::AlwaysAllow,
            this_machine: true,
        }
    }

    pub fn set_use(&self, computer: &str, value: ComputerUse) {
        FakeComputer::guard(&self.uses).insert(computer.to_owned(), value);
    }

    pub fn set_policy(&self, computer: &str, value: LocalPolicy) {
        FakeComputer::guard(&self.policies).insert(computer.to_owned(), value);
    }

    #[must_use]
    pub fn use_of(&self, computer: &str) -> ComputerUse {
        FakeComputer::guard(&self.uses)
            .get(computer)
            .copied()
            .unwrap_or_else(Self::default_use)
    }

    pub(crate) fn gate_modes(&self) -> ModeFlags {
        self.computer.gate_config().modes.clone()
    }
}

impl TrustSettings for FakeTrust {
    fn computer_use(
        &self,
        _bot: &BotId,
        computer: &str,
    ) -> Result<ComputerUse, TrustSettingsError> {
        Ok(self.use_of(computer))
    }

    fn policy(&self, computer: &str) -> Result<LocalPolicy, TrustSettingsError> {
        Ok(FakeComputer::guard(&self.policies)
            .get(computer)
            .copied()
            .unwrap_or_else(LocalPolicy::unset))
    }

    fn modes(&self) -> Result<ModeFlags, TrustSettingsError> {
        Ok(self.gate_modes())
    }

    fn user_rules(
        &self,
        _bot: &BotId,
        conversation: ConversationKey,
    ) -> Result<UserRules, TrustSettingsError> {
        let mut rules = self.computer.gate_config().user.clone();
        if let Some(keys) = FakeComputer::guard(&self.conversation_allowed).get(&conversation) {
            rules.session.extend(keys.iter().cloned());
        }
        Ok(rules)
    }
}
