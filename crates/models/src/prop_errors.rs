use std::sync::Arc;

use proptest::prelude::*;

use crate::egress::DataPolicy;
use crate::errors::{FailInput, RetryPlan, classify};
use crate::prop_run::{ask_one, hi, model_ref, poster_of};
use crate::scripted::{ScriptStep, connect_fail, http_fail, session_with};
use crate::types::Audience;

fn fail_step(kind: u8) -> ScriptStep {
    match kind % 8 {
        0 => http_fail(400, "prompt is too long"),
        1 => http_fail(401, "no"),
        2 => http_fail(404, "missing"),
        3 => http_fail(402, "quota"),
        4 => http_fail(418, "teapot"),
        5 => http_fail(500, "down"),
        6 => http_fail(429, "busy"),
        _ => connect_fail(),
    }
}

fn input_of(kind: u8) -> FailInput {
    match kind % 8 {
        0 => FailInput::Http {
            status: 400,
            message: "prompt is too long".to_owned(),
        },
        1 => FailInput::Http {
            status: 401,
            message: "no".to_owned(),
        },
        2 => FailInput::Http {
            status: 404,
            message: "missing".to_owned(),
        },
        3 => FailInput::Http {
            status: 402,
            message: "quota".to_owned(),
        },
        4 => FailInput::Http {
            status: 418,
            message: "teapot".to_owned(),
        },
        5 => FailInput::Http {
            status: 500,
            message: "down".to_owned(),
        },
        6 => FailInput::Http {
            status: 429,
            message: "busy".to_owned(),
        },
        _ => FailInput::Connect { ollama: false },
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    #[test]
    fn t_models_errors_4(
        (kinds, watching) in (
            proptest::collection::vec(0_u8..8, 1..9),
            any::<bool>(),
        )
    ) {
        let audience = if watching { Audience::Watching } else { Audience::Unattended };
        let limit = crate::retry::try_limit(audience);
        let mut steps = vec![crate::scripted::ok_text("done")];
        for kind in kinds.iter().rev() {
            steps.push(fail_step(*kind));
        }
        let poster = poster_of(steps);
        let sent_poster = Arc::clone(&poster);
        let mut session = session_with(poster);
        let primary = model_ref(crate::prop_run::custom("m", crate::types::WireFormat::OpenAiCompatible));
        let request = hi();
        let policy = DataPolicy::LocalOnly;
        let mut plan = crate::session::ask_plan(&primary, &[], &policy, &request, audience);
        let _outcome = ask_one(&mut session, &mut plan);
        let sent = sent_poster.sent();
        let count = u32::try_from(sent.len()).unwrap_or(u32::MAX);
        prop_assert!(count <= limit);
        let first = kinds.first().copied().unwrap_or(0);
        if matches!(classify(&input_of(first)).retry, RetryPlan::None) {
            prop_assert_eq!(sent.len(), 1);
            prop_assert!(sent.first().is_some_and(|row| !row.carries().is_empty() || first > 6));
        }
    }
}
