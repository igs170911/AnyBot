use std::sync::Arc;

use proptest::prelude::*;

use crate::prop_run::{ask_one, hi, model_ref, poster_of};
use crate::scripted::ScriptStep;
use crate::types::Audience;

fn blank_body() -> String {
    "{\"choices\":[{\"message\":{\"role\":\"assistant\",\"content\":\"\"},\"finish_reason\":\"stop\"}]}".to_owned()
}

fn reject_body() -> String {
    "{\"error\":{\"message\":\"unknown parameter thinking\"}}".to_owned()
}

fn step_of(kind: u8, stream: bool) -> ScriptStep {
    match kind % 5 {
        0 => blank_step(stream),
        1 => crate::scripted::script_http(400, &reject_body()),
        2 => crate::scripted::script_http(408, "slow"),
        3 => cut_before_text(stream),
        _ => crate::scripted::connect_fail(),
    }
}

fn blank_step(stream: bool) -> ScriptStep {
    if stream {
        return crate::scripted::script_chunks(
            200,
            &[
                "data: {\"choices\":[{\"delta\":{},\"finish_reason\":\"stop\"}]}\n\n".to_owned(),
                "data: [DONE]\n\n".to_owned(),
            ],
        );
    }
    crate::scripted::script_http(200, &blank_body())
}

fn cut_before_text(stream: bool) -> ScriptStep {
    if stream {
        return crate::scripted::script_chunks(
            200,
            &["data: {\"choices\":[{\"delta\":{}}]}\n".to_owned()],
        );
    }
    crate::scripted::script_cut(200, "{\"choices\":[", 4)
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

fn distinct_rejects(count: u8) -> Vec<ScriptStep> {
    (0..count)
        .map(|index| {
            let body = format!("{{\"error\":{{\"message\":\"unknown parameter p{index}\"}}}}");
            crate::scripted::script_http(400, &body)
        })
        .collect()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    #[test]
    fn t_models_errors_5(
        (pattern, watching, extras, stream) in (0_u8..5, any::<bool>(), 0_u8..4, any::<bool>())
    ) {
        let audience = if watching { Audience::Watching } else { Audience::Unattended };
        let limit = if watching { 3_u32 } else { 8_u32 };
        let mut steps = vec![done_step(stream)];
        if pattern == 2 {
            steps.extend(distinct_rejects(extras).into_iter().rev());
        } else {
            for _ in 0..12 {
                steps.push(step_of(pattern, stream));
            }
        }
        let poster = poster_of(steps);
        let sent_poster = Arc::clone(&poster);
        let mut session = crate::scripted::session_with(poster);
        let primary = model_ref(crate::prop_run::custom("m", crate::types::WireFormat::OpenAiCompatible));
        let request = hi();
        let mut plan = crate::session::ask_plan(&primary, &[], &request, audience);
        plan.stream = stream;
        let outcome = ask_one(&mut session, &mut plan);
        let sent = u32::try_from(sent_poster.sent().len()).unwrap_or(u32::MAX);
        let dropped = u32::from(extras);
        prop_assert!(sent <= limit.saturating_add(dropped).saturating_add(1));
        if pattern == 0 {
            prop_assert!(sent <= 2);
        }
        if pattern == 1 {
            prop_assert!(sent <= limit.saturating_add(1));
        }
        if matches!(pattern, 3 | 4) {
            prop_assert!(outcome.is_err());
            let shown = format!("{outcome:?}");
            prop_assert!(!shown.contains("text: \"\""));
        }
        if pattern == 3 {
            prop_assert!(outcome.is_err());
            prop_assert!(sent <= limit.saturating_add(1));
        }
        if stream {
            prop_assert!(sent_poster.sent().iter().any(|row| row.carries().contains("\"stream\":true")));
        }
    }
}
