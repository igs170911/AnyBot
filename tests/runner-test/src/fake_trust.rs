use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use anybot_foundation::BotId;
use anybot_runner::{
    ComputerUseChange, ConversationKey, TrustSettings, TrustSettingsError, YoloState,
};
use anybot_trust::{
    ActionKey, AlwaysEntry, ComputerUse, LocalPolicy, ModeFlags, RunHere, UserRules,
};

use crate::fake_computer::FakeComputer;

pub struct FakeTrust {
    pub(crate) computer: Arc<FakeComputer>,
    pub(crate) uses: Mutex<BTreeMap<String, ComputerUse>>,
    pub(crate) policies: Mutex<BTreeMap<String, LocalPolicy>>,
    pub(crate) conversation_allowed: Mutex<BTreeMap<ConversationKey, Vec<ActionKey>>>,
    pub(crate) unreadable: Mutex<Option<String>>,
    pub(crate) yolo: Mutex<YoloState>,
    pub(crate) smart_raw: Mutex<i64>,
}

impl FakeTrust {
    #[must_use]
    pub fn new(computer: Arc<FakeComputer>) -> Self {
        FakeTrust {
            computer,
            uses: Mutex::new(BTreeMap::new()),
            policies: Mutex::new(BTreeMap::new()),
            conversation_allowed: Mutex::new(BTreeMap::new()),
            unreadable: Mutex::new(None),
            yolo: Mutex::new(YoloState::Off),
            smart_raw: Mutex::new(300),
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

    pub fn set_smart_seconds(&self, raw: i64) {
        *FakeComputer::guard(&self.smart_raw) = raw;
    }

    pub fn set_unreadable(&self, rule: Option<&str>) {
        *FakeComputer::guard(&self.unreadable) = rule.map(ToOwned::to_owned);
    }

    #[must_use]
    pub fn always_entries(&self) -> Vec<AlwaysEntry> {
        self.computer.gate_config().user.always.entries.clone()
    }

    pub fn remove_always(&self, action: &str) {
        self.computer.gate_config().user.always.remove(action);
    }

    #[must_use]
    pub fn ask_rule_count(&self) -> usize {
        self.computer.gate_config().user.ask_style.len()
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
        if let Some(rule) = FakeComputer::guard(&self.unreadable).clone() {
            return Err(TrustSettingsError::Unreadable { rule });
        }
        let mut rules = self.computer.gate_config().user.clone();
        if let Some(keys) = FakeComputer::guard(&self.conversation_allowed).get(&conversation) {
            rules.session.extend(keys.iter().cloned());
        }
        Ok(rules)
    }

    fn allow_in_conversation(
        &self,
        _bot: &BotId,
        conversation: ConversationKey,
        key: &ActionKey,
    ) -> Result<(), TrustSettingsError> {
        FakeComputer::guard(&self.conversation_allowed)
            .entry(conversation)
            .or_default()
            .push(key.clone());
        Ok(())
    }

    fn add_always(&self, entry: AlwaysEntry) -> Result<(), TrustSettingsError> {
        let mut gate = self.computer.gate_config();
        if !gate.user.always.contains(&entry.action) {
            gate.user.always.add(entry);
        }
        Ok(())
    }

    fn remove_ask_rule(&self, rule: &str) -> Result<(), TrustSettingsError> {
        self.computer.gate_config().user.ask_style.retain(|key| {
            let (tool, subject) = key.parts();
            format!("{tool}({subject})") != rule
        });
        Ok(())
    }

    fn set_computer_use(
        &self,
        _bot: &BotId,
        computer: &str,
        change: ComputerUseChange,
    ) -> Result<(), TrustSettingsError> {
        let mut value = self.use_of(computer);
        match change {
            ComputerUseChange::AllowBot => value.usable_by_bot = true,
            ComputerUseChange::RunHereAlways => value.run_here = RunHere::AlwaysAllow,
        }
        self.set_use(computer, value);
        Ok(())
    }

    fn set_direct_mode(&self, on: bool) -> Result<(), TrustSettingsError> {
        self.computer.gate_config().modes.direct_mode = on;
        Ok(())
    }

    fn smart_seconds(&self) -> Result<u32, TrustSettingsError> {
        Ok(anybot_trust::smart_timeout::smart_seconds(
            *FakeComputer::guard(&self.smart_raw),
        ))
    }

    fn yolo(&self) -> Result<YoloState, TrustSettingsError> {
        Ok(*FakeComputer::guard(&self.yolo))
    }

    fn set_yolo(&self, state: YoloState) -> Result<(), TrustSettingsError> {
        *FakeComputer::guard(&self.yolo) = state;
        self.computer.gate_config().modes.yolo = state != YoloState::Off;
        Ok(())
    }
}
