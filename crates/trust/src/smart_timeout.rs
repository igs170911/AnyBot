use anybot_protocol::ComputerOwner;

use crate::card_buttons::AskOrigin;

const MIN_SECONDS: i64 = 30;
const MAX_SECONDS: i64 = 3600;
const GRID: i64 = 30;

#[must_use]
pub fn smart_seconds(raw: i64) -> u32 {
    let clamped = raw.clamp(MIN_SECONDS, MAX_SECONDS);
    let on_grid = clamped.div_euclid(GRID).saturating_mul(GRID);
    u32::try_from(on_grid).unwrap_or(u32::MAX)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolRisk {
    LowRisk,
    BotWork,
    Sensitive,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmartChoice {
    AutoAllow,
    AutoDeny,
    Never,
}

#[must_use]
pub fn tool_risk(tool: &str, owner: ComputerOwner) -> ToolRisk {
    let on_bot = owner == ComputerOwner::Bot;
    match tool {
        "read_file" => ToolRisk::LowRisk,
        "shell" => ToolRisk::BotWork,
        "write_file" | "edit_file" if on_bot => ToolRisk::BotWork,
        _ => ToolRisk::Unknown,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmartCase<'a> {
    pub origin: &'a AskOrigin,
    pub risk: ToolRisk,
    pub has_classes: bool,
    pub is_switch: bool,
    pub mdm_locked: bool,
}

#[must_use]
pub fn smart_choice(case: &SmartCase<'_>) -> SmartChoice {
    if case.mdm_locked || case.has_classes || case.is_switch {
        return SmartChoice::Never;
    }
    if *case.origin != AskOrigin::Default {
        return SmartChoice::Never;
    }
    match case.risk {
        ToolRisk::LowRisk | ToolRisk::BotWork => SmartChoice::AutoAllow,
        ToolRisk::Sensitive => SmartChoice::AutoDeny,
        ToolRisk::Unknown => SmartChoice::Never,
    }
}

#[cfg(test)]
#[path = "smart_timeout_tests.rs"]
mod smart_timeout_tests;
