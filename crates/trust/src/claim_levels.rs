use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ClaimLevel {
    Unknown,
    Inferred,
    SourceBacked,
    Verified,
}

impl fmt::Display for ClaimLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            ClaimLevel::Unknown => "unknown",
            ClaimLevel::Inferred => "inferred",
            ClaimLevel::SourceBacked => "source-backed",
            ClaimLevel::Verified => "verified",
        };
        f.write_str(text)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceKind {
    ToolReturn,
    UserWords,
    BotHearsay,
    ModelDerivation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    BotComputer,
    UserComputer,
    Phone,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Link {
    pub kind: EvidenceKind,
    pub gate_checked: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Verification {
    pub scope: Scope,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Claim {
    pub text: String,
    pub links: Vec<Link>,
    pub verification: Option<Verification>,
    pub claim_scope: Option<Scope>,
}

#[must_use]
pub fn claim_level(claim: &Claim) -> ClaimLevel {
    if claim.links.is_empty() {
        return ClaimLevel::Unknown;
    }
    let covers = claim
        .verification
        .as_ref()
        .is_some_and(|v| claim.claim_scope.is_some_and(|s| v.scope == s));
    let all_gated = claim.links.iter().all(|l| l.gate_checked);
    if covers && all_gated {
        return ClaimLevel::Verified;
    }
    let has_source = claim
        .links
        .iter()
        .any(|l| matches!(l.kind, EvidenceKind::ToolReturn | EvidenceKind::UserWords));
    if has_source {
        return ClaimLevel::SourceBacked;
    }
    ClaimLevel::Inferred
}

#[must_use]
pub fn to_report_level(level: ClaimLevel) -> anybot_protocol::ReportLevel {
    match level {
        ClaimLevel::Unknown => anybot_protocol::ReportLevel::Unknown,
        ClaimLevel::Inferred => anybot_protocol::ReportLevel::Inferred,
        ClaimLevel::SourceBacked => anybot_protocol::ReportLevel::Sourced,
        ClaimLevel::Verified => anybot_protocol::ReportLevel::Verified,
    }
}

#[must_use]
pub fn unverified_scopes(claim: &Claim) -> Vec<Scope> {
    let covered = claim.verification.as_ref().map(|v| v.scope);
    [Scope::BotComputer, Scope::UserComputer, Scope::Phone]
        .iter()
        .filter(|s| covered != Some(**s))
        .copied()
        .collect()
}
