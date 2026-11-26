use std::sync::Arc;

use proptest::prelude::*;

use crate::prop_run::{ask_one, hi, model_ref, poster_of};
use crate::scripted::{ScriptStep, connect_fail, http_fail, session_with};
use crate::types::Audience;

fn fail_step(kind: u8, stream: bool) -> ScriptStep {
    match kind % 13 {
        0 => http_fail(400, "prompt is too long"),
        1 => http_fail(401, "no"),
        2 => http_fail(404, "missing"),
        3 => http_fail(402, "quota"),
        4 => http_fail(418, "teapot"),
        5 => http_fail(500, "down"),
        6 => http_fail(429, "busy"),
        7 => connect_fail(),
        8 => blank_for(stream),
        9 => crate::scripted::reject_step(),
        10 => crate::scripted::first_token_step(),
        11 => crate::scripted::stall_step(),
        _ => mixed_step(stream),
    }
}

fn mixed_step(stream: bool) -> ScriptStep {
    if stream {
        return crate::scripted::script_chunks(
            200,
            &[
                "data: {\"choices\":[{\"delta\":{\"content\":\"x\"}}]}\n\n".to_owned(),
                "data: {\"error\":{\"message\":\"broke\"}}\n\n".to_owned(),
            ],
        );
    }
    http_fail(418, "other")
}

fn done_step(stream: bool) -> ScriptStep {
    if stream {
        return crate::scripted::script_chunks(
            200,
            &[
                "data: {\"choices\":[{\"delta\":{\"content\":\"done\"}}]}\n\n".to_owned(),
                "data: [DONE]\n\n".to_owned(),
            ],
        );
    }
    crate::scripted::ok_text("done")
}

fn blank_for(stream: bool) -> ScriptStep {
    if stream {
        return crate::scripted::script_chunks(
            200,
            &[
                "data: {\"choices\":[{\"delta\":{},\"finish_reason\":\"stop\"}]}\n\n".to_owned(),
                "data: [DONE]\n\n".to_owned(),
            ],
        );
    }
    crate::scripted::blank_step()
}

fn once_only(kind: u8) -> bool {
    matches!(kind % 13, 0 | 1 | 2 | 3 | 4 | 11 | 12)
}

fn extras_of(kinds: &[u8]) -> u32 {
    let blanks = kinds.iter().filter(|kind| *kind % 13 == 8).count();
    let drops = kinds.iter().filter(|kind| *kind % 13 == 9).count();
    u32::try_from(blanks.saturating_add(drops)).unwrap_or(1)
}

fn retry_free(session: &crate::session::Session) -> bool {
    session
        .failure_logs()
        .iter()
        .all(|row| row.code != "model-retry-wait")
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    #[test]
    fn t_models_errors_4(
        (kinds, watching, stream) in (
            proptest::collection::vec(0_u8..13, 1..9),
            any::<bool>(),
            any::<bool>(),
        )
    ) {
        let audience = if watching { Audience::Watching } else { Audience::Unattended };
        let limit = if watching { 3_u32 } else { 8_u32 };
        let mut steps = vec![done_step(stream)];
        for kind in kinds.iter().rev() {
            steps.push(fail_step(*kind, stream));
        }
        let poster = poster_of(steps);
        let sent_poster = Arc::clone(&poster);
        let mut session = session_with(poster);
        let primary = model_ref(crate::prop_run::custom("m", crate::types::WireFormat::OpenAiCompatible));
        let request = hi();
        let mut plan = crate::session::ask_plan(&primary, &[], &request, audience);
        plan.stream = stream;
        let _outcome = ask_one(&mut session, &mut plan);
        let sent = sent_poster.sent();
        let count = u32::try_from(sent.len()).unwrap_or(u32::MAX);
        prop_assert!(count <= limit.saturating_add(extras_of(&kinds)));
        let no_retry_run = kinds.iter().take_while(|kind| once_only(**kind)).count();
        if no_retry_run > 0 && kinds.first().is_some_and(|kind| kind % 13 != 12) {
            prop_assert_eq!(sent.len(), 1);
        }
        if kinds.iter().any(|kind| kind % 13 == 12) && stream {
            prop_assert!(sent.iter().any(|row| row.carries().contains("\"stream\":true")));
        }
        if kinds.first().is_some_and(|kind| kind % 13 == 8)
            && kinds.get(1).is_none_or(|kind| once_only(*kind))
        {
            prop_assert!(sent.len() <= 2);
        }
        if kinds.first().is_some_and(|kind| kind % 13 == 9) {
            prop_assert!(retry_free(&session));
        }
    }
}
