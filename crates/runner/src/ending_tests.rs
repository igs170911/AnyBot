use anybot_models::{Classified, FailInput, FallbackPlan, GatewayFail, RetryPlan, classify};
use anybot_protocol::TurnEndReason;

use crate::ending::end_reason;

fn classified(input: &FailInput) -> GatewayFail {
    GatewayFail::Classified {
        classified: classify(input),
        notices: Vec::new(),
    }
}

fn failed(code: &str) -> TurnEndReason {
    TurnEndReason::ModelFailed {
        code: code.to_owned(),
    }
}

#[test]
fn end_reason_from_gateway_fail() {
    assert_eq!(
        end_reason(&classified(&FailInput::Budget)),
        TurnEndReason::ContextBudget {
            deficit_tokens: None
        }
    );
    assert_eq!(
        end_reason(&classified(&FailInput::FirstToken)),
        failed("model-first-token")
    );
    assert_eq!(
        end_reason(&classified(&FailInput::Http {
            status: 401,
            message: "invalid key".to_owned(),
        })),
        failed("model-auth")
    );
    let blank = Classified {
        kind: anybot_models::FailKind::Other4xx,
        retry: RetryPlan::None,
        fallback: FallbackPlan::Skip,
        code: "model-blank",
        http_status: None,
        limit: None,
        retry_after: None,
    };
    assert_eq!(
        end_reason(&GatewayFail::Classified {
            classified: blank,
            notices: Vec::new(),
        }),
        failed("model-blank")
    );
    for fail in [GatewayFail::Stopped, GatewayFail::MissingKey] {
        assert_eq!(end_reason(&fail), failed("turn-model-failed"));
    }
    let exhausted = GatewayFail::Exhausted {
        fallback: None,
        classified: classify(&FailInput::Http {
            status: 503,
            message: "overloaded".to_owned(),
        }),
        notices: Vec::new(),
    };
    assert_eq!(end_reason(&exhausted), failed("model-server"));
}
