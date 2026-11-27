pub const CLAIM_WINDOW_MS: u64 = 10_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LetterKind {
    Round,
    CardAnswer,
}

impl LetterKind {
    #[must_use]
    pub fn code(self) -> &'static str {
        match self {
            LetterKind::Round => "round",
            LetterKind::CardAnswer => "card-answer",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LetterState {
    Pending,
    Claimed,
    Reassigned,
    Retracted,
}

impl LetterState {
    #[must_use]
    pub fn code(self) -> &'static str {
        match self {
            LetterState::Pending => "pending",
            LetterState::Claimed => "claimed",
            LetterState::Reassigned => "reassigned",
            LetterState::Retracted => "retracted",
        }
    }

    #[must_use]
    pub fn from_code(code: &str) -> Option<LetterState> {
        match code {
            "pending" => Some(LetterState::Pending),
            "claimed" => Some(LetterState::Claimed),
            "reassigned" => Some(LetterState::Reassigned),
            "retracted" => Some(LetterState::Retracted),
            _ => None,
        }
    }
}

#[must_use]
pub fn claim_deadline_ms(created_ms: u64, delivered_ms: Option<u64>) -> u64 {
    delivered_ms
        .unwrap_or(created_ms)
        .saturating_add(CLAIM_WINDOW_MS)
}

#[must_use]
pub fn claim_expired(created_ms: u64, delivered_ms: Option<u64>, now_ms: u64) -> bool {
    now_ms >= claim_deadline_ms(created_ms, delivered_ms)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClaimRefusal {
    Reassigned,
    TakenByOther,
    AlreadyYours,
    Retracted,
}

#[must_use]
pub fn refusal_for(state: LetterState, claimed_by: Option<&str>, me: &str) -> Option<ClaimRefusal> {
    match state {
        LetterState::Pending => None,
        LetterState::Reassigned => Some(ClaimRefusal::Reassigned),
        LetterState::Retracted => Some(ClaimRefusal::Retracted),
        LetterState::Claimed if claimed_by == Some(me) => Some(ClaimRefusal::AlreadyYours),
        LetterState::Claimed => Some(ClaimRefusal::TakenByOther),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ten_seconds_run_from_the_delivery_or_else_from_the_creation() {
        assert!(!claim_expired(1_000, Some(5_000), 14_999));
        assert!(claim_expired(1_000, Some(5_000), 15_000));
        assert!(!claim_expired(1_000, None, 10_999));
        assert!(claim_expired(1_000, None, 11_000));
    }

    #[test]
    fn each_state_has_its_own_refusal() {
        assert_eq!(refusal_for(LetterState::Pending, None, "a"), None);
        assert_eq!(
            refusal_for(LetterState::Reassigned, None, "a"),
            Some(ClaimRefusal::Reassigned)
        );
        assert_eq!(
            refusal_for(LetterState::Retracted, None, "a"),
            Some(ClaimRefusal::Retracted)
        );
        assert_eq!(
            refusal_for(LetterState::Claimed, Some("a"), "a"),
            Some(ClaimRefusal::AlreadyYours)
        );
        assert_eq!(
            refusal_for(LetterState::Claimed, Some("b"), "a"),
            Some(ClaimRefusal::TakenByOther)
        );
    }

    #[test]
    fn state_codes_round_trip() {
        for state in [
            LetterState::Pending,
            LetterState::Claimed,
            LetterState::Reassigned,
            LetterState::Retracted,
        ] {
            assert_eq!(LetterState::from_code(state.code()), Some(state));
        }
        assert_eq!(LetterState::from_code("other"), None);
    }
}
