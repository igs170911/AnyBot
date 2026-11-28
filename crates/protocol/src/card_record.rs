use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum CardKind {
    Action,
    EveryTime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum CardButton {
    AllowOnce,
    AllowConversation,
    AlwaysAllow,
    Deny,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn card_kind_and_button_round_trip() {
        for kind in [CardKind::Action, CardKind::EveryTime] {
            let text = serde_json::to_string(&kind).unwrap();
            let back: CardKind = serde_json::from_str(&text).unwrap();
            assert_eq!(back, kind);
        }
        for button in [
            CardButton::AllowOnce,
            CardButton::AllowConversation,
            CardButton::AlwaysAllow,
            CardButton::Deny,
        ] {
            let text = serde_json::to_string(&button).unwrap();
            let back: CardButton = serde_json::from_str(&text).unwrap();
            assert_eq!(back, button);
        }
    }
}
