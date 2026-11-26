use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleLayer {
    Floor,
    Admin,
    User,
    Default,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GateOutcome {
    Allow,
    Ask,
    Deny,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct GateNo(pub u8);

impl GateNo {
    #[must_use]
    pub fn value(self) -> u8 {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateRecord {
    pub gate: GateNo,
    pub layer: RuleLayer,
    pub rule: String,
    pub source: JudgementSource,
    pub outcome: GateOutcome,
    pub read_only: bool,
}

impl GateRecord {
    #[must_use]
    pub fn new(
        gate: GateNo,
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
            GateNo(1),
            RuleLayer::Floor,
            "sandbox-deny".to_owned(),
            JudgementSource::Rule,
            GateOutcome::Deny,
            false,
        );
        assert_eq!(record.gate.value(), 1);
        assert_eq!(record.layer, RuleLayer::Floor);
        assert_eq!(record.rule, "sandbox-deny");
        assert_eq!(record.source, JudgementSource::Rule);
        assert_eq!(record.outcome, GateOutcome::Deny);
        assert!(!record.read_only);
    }
}
