use anybot_foundation as _;
use anybot_models::Audience;
use bytes as _;
use futures_util as _;
use proptest as _;
use reqwest as _;
use rustls as _;
use serde as _;
use serde_json as _;
use test_harness as _;
use tiktoken_rs as _;
use tokio as _;
use tokio_rustls as _;
use trybuild as _;
use url as _;

#[test]
fn errors_property_never_retries_terminal_kinds() {
    let kinds = [
        anybot_models::FailInput::MissingKey,
        anybot_models::FailInput::Http {
            status: 401,
            message: "no".to_owned(),
        },
        anybot_models::FailInput::Http {
            status: 404,
            message: "missing".to_owned(),
        },
        anybot_models::FailInput::Stalled,
    ];
    for _ in 0..250 {
        for input in &kinds {
            let row = anybot_models::classify(input);
            assert_eq!(row.retry, anybot_models::RetryPlan::None);
        }
    }
    assert!(anybot_models::try_limit(Audience::Watching) <= 3);
    assert!(anybot_models::try_limit(Audience::Unattended) <= 8);
}
