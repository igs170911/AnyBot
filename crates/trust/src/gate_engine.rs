use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateLayer {
    Floor,
    Admin,
    User,
    Default,
}

impl fmt::Display for GateLayer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            GateLayer::Floor => "floor",
            GateLayer::Admin => "admin",
            GateLayer::User => "user",
            GateLayer::Default => "default",
        };
        f.write_str(text)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Opinion {
    None,
    Deny,
    Ask,
    Allow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Deny,
    Ask,
    Allow,
}

impl Outcome {
    #[must_use]
    pub fn from_opinion(opinion: Opinion) -> Option<Self> {
        match opinion {
            Opinion::None => None,
            Opinion::Deny => Some(Outcome::Deny),
            Opinion::Ask => Some(Outcome::Ask),
            Opinion::Allow => Some(Outcome::Allow),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LayerCall {
    pub layer: GateLayer,
    pub deny: bool,
    pub ask: bool,
    pub allow: bool,
}

impl LayerCall {
    #[must_use]
    pub fn opinion(&self) -> Opinion {
        if self.deny {
            return Opinion::Deny;
        }
        if self.ask {
            return Opinion::Ask;
        }
        if self.allow {
            return Opinion::Allow;
        }
        Opinion::None
    }
}

pub const LAYER_ORDER: [GateLayer; 4] = [
    GateLayer::Floor,
    GateLayer::Admin,
    GateLayer::User,
    GateLayer::Default,
];

#[must_use]
pub fn precedence_decision(calls: &[LayerCall]) -> Option<(GateLayer, Outcome)> {
    for layer in LAYER_ORDER {
        let call = calls.iter().find(|c| c.layer == layer);
        let Some(call) = call else { continue };
        let opinion = call.opinion();
        if let Some(outcome) = Outcome::from_opinion(opinion) {
            return Some((layer, outcome));
        }
    }
    None
}
