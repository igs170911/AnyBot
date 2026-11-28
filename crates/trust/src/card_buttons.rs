use anybot_protocol::{CardButton, RuleLayer};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AskOrigin {
    Default,
    EveryTime,
    UserAskRule { rule: String },
    UserHook,
    AdminOrFloor { layer: RuleLayer },
    Untrusted,
}

#[must_use]
pub fn button_set(origin: &AskOrigin) -> Vec<CardButton> {
    match origin {
        AskOrigin::Default => vec![
            CardButton::AllowOnce,
            CardButton::AllowConversation,
            CardButton::AlwaysAllow,
            CardButton::Deny,
        ],
        AskOrigin::UserAskRule { .. } => vec![
            CardButton::AllowOnce,
            CardButton::AlwaysAllow,
            CardButton::Deny,
        ],
        AskOrigin::EveryTime
        | AskOrigin::UserHook
        | AskOrigin::AdminOrFloor { .. }
        | AskOrigin::Untrusted => vec![CardButton::AllowOnce, CardButton::Deny],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn button_set_matches_answer_table() {
        assert_eq!(
            button_set(&AskOrigin::Default),
            vec![
                CardButton::AllowOnce,
                CardButton::AllowConversation,
                CardButton::AlwaysAllow,
                CardButton::Deny,
            ]
        );
        assert_eq!(
            button_set(&AskOrigin::EveryTime),
            vec![CardButton::AllowOnce, CardButton::Deny]
        );
        assert_eq!(
            button_set(&AskOrigin::UserAskRule {
                rule: "shell(make*)".to_owned(),
            }),
            vec![
                CardButton::AllowOnce,
                CardButton::AlwaysAllow,
                CardButton::Deny,
            ]
        );
        assert_eq!(
            button_set(&AskOrigin::UserHook),
            vec![CardButton::AllowOnce, CardButton::Deny]
        );
        assert_eq!(
            button_set(&AskOrigin::AdminOrFloor {
                layer: RuleLayer::Admin,
            }),
            vec![CardButton::AllowOnce, CardButton::Deny]
        );
        assert_eq!(
            button_set(&AskOrigin::AdminOrFloor {
                layer: RuleLayer::Floor,
            }),
            vec![CardButton::AllowOnce, CardButton::Deny]
        );
        assert_eq!(
            button_set(&AskOrigin::Untrusted),
            vec![CardButton::AllowOnce, CardButton::Deny]
        );
    }
}
