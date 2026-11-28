use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum RuleLayer {
    Floor,
    Admin,
    User,
    Default,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum GateOutcome {
    Allow,
    Ask,
    Deny,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum JudgementSource {
    Rule,
    ProgramCheck,
    ModelJudgement,
    You,
    AlwaysApprove,
    RuleTimeout,
    Ungated,
    ForwardedUnverified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum GateStep {
    Initiator = 0,
    Computer = 1,
    Floor = 2,
    Admin = 3,
    ComputerUse = 4,
    UserHooks = 5,
    AskPatterns = 6,
    AllowLists = 7,
    DefaultRule = 8,
    Relax = 9,
    AskYou = 10,
}

impl GateStep {
    #[must_use]
    pub const fn number(self) -> u8 {
        self as u8
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GateStepError(pub u8);

impl fmt::Display for GateStepError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid gate step: {}", self.0)
    }
}

impl std::error::Error for GateStepError {}

impl TryFrom<u8> for GateStep {
    type Error = GateStepError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(GateStep::Initiator),
            1 => Ok(GateStep::Computer),
            2 => Ok(GateStep::Floor),
            3 => Ok(GateStep::Admin),
            4 => Ok(GateStep::ComputerUse),
            5 => Ok(GateStep::UserHooks),
            6 => Ok(GateStep::AskPatterns),
            7 => Ok(GateStep::AllowLists),
            8 => Ok(GateStep::DefaultRule),
            9 => Ok(GateStep::Relax),
            10 => Ok(GateStep::AskYou),
            other => Err(GateStepError(other)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
pub struct GateRecord {
    pub gate: GateStep,
    pub layer: RuleLayer,
    pub rule: String,
    pub source: JudgementSource,
    pub outcome: GateOutcome,
    pub read_only: bool,
}

impl GateRecord {
    #[must_use]
    pub fn new(
        gate: GateStep,
        layer: RuleLayer,
        rule: String,
        source: JudgementSource,
        outcome: GateOutcome,
        read_only: bool,
    ) -> Self {
        GateRecord {
            gate,
            layer,
            rule,
            source,
            outcome,
            read_only,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gate_record_keeps_gate_number_layer_rule_source_outcome() {
        let record = GateRecord::new(
            GateStep::Floor,
            RuleLayer::Floor,
            "sandbox-deny".to_owned(),
            JudgementSource::Rule,
            GateOutcome::Deny,
            false,
        );
        assert_eq!(record.gate.number(), 2);
        assert_eq!(record.layer, RuleLayer::Floor);
        assert_eq!(record.rule, "sandbox-deny");
        assert_eq!(record.source, JudgementSource::Rule);
        assert_eq!(record.outcome, GateOutcome::Deny);
        assert!(!record.read_only);
    }

    #[test]
    fn gate_step_rejects_eleven() {
        for n in 0_u8..=10 {
            let step = GateStep::try_from(n).expect("0..=10 must parse");
            assert_eq!(step.number(), n);
        }
        assert_eq!(GateStep::try_from(11), Err(GateStepError(11)));
        assert_eq!(GateStep::try_from(255), Err(GateStepError(255)));
    }
}
