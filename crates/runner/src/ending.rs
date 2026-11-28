use anybot_models::{Classified, FailKind, GatewayFail};
use anybot_protocol::TurnEndReason;

pub const CODE_MODEL_FAILED: &str = "turn-model-failed";

#[must_use]
pub fn end_reason(fail: &GatewayFail) -> TurnEndReason {
    match fail {
        GatewayFail::Classified { classified, .. } | GatewayFail::Exhausted { classified, .. } => {
            classified_reason(classified)
        }
        GatewayFail::MissingKey
        | GatewayFail::KeyDestination
        | GatewayFail::Status(_)
        | GatewayFail::Transport(_)
        | GatewayFail::Stopped => TurnEndReason::ModelFailed {
            code: CODE_MODEL_FAILED.to_owned(),
        },
    }
}

fn classified_reason(classified: &Classified) -> TurnEndReason {
    if classified.kind == FailKind::Budget {
        return TurnEndReason::ContextBudget {
            deficit_tokens: None,
        };
    }
    TurnEndReason::ModelFailed {
        code: classified.code.to_owned(),
    }
}

#[must_use]
pub fn end_event(
    reason: TurnEndReason,
    reply_delivered: bool,
) -> anybot_protocol::KnownTaskEventKind {
    anybot_protocol::KnownTaskEventKind::TurnEnded {
        reason,
        reply_delivered,
    }
}

#[cfg(test)]
#[path = "ending_tests.rs"]
mod ending_tests;
