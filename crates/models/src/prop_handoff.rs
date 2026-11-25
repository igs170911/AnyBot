use std::sync::{Arc, Mutex};

use proptest::prelude::*;

use crate::handoff::SwitchReason;
use crate::prop_run::{ask_one, hi, model_ref};
use crate::scripted::{ScriptPoster, ok_text, session_with};
use crate::session::{ModelRef, SwitchHandle};
use crate::stream_fail::sse_payloads;
use crate::types::{Audience, ProviderKind, WireFormat};

#[derive(Clone, Debug)]
struct Step {
    stream: bool,
    switch: bool,
    reason: u8,
}

struct Armed {
    handle: Option<SwitchHandle>,
    target: Option<ModelRef>,
    reason: SwitchReason,
}

struct AskParts<'a> {
    session: &'a mut crate::session::Session,
    poster: &'a ScriptPoster,
    armed: &'a Arc<Mutex<Armed>>,
    primary: &'a ModelRef,
    other: &'a ModelRef,
    request: &'a crate::types::TurnRequest,
}

fn step_strategy() -> impl Strategy<Value = Vec<Step>> {
    proptest::collection::vec(
        (any::<bool>(), any::<bool>(), 0_u8..3).prop_map(|(stream, switch, reason)| Step {
            stream,
            switch,
            reason,
        }),
        2..6,
    )
}

fn reason_of(code: u8) -> SwitchReason {
    match code % 3 {
        0 => SwitchReason::User,
        1 => SwitchReason::Bot,
        _ => SwitchReason::Fallback,
    }
}

fn sse_events() -> Vec<String> {
    let events = [
        r#"{"choices":[{"delta":{"content":"hel"},"finish_reason":null}]}"#,
        r#"{"choices":[{"delta":{"content":"lo"},"finish_reason":"stop"}]}"#,
    ];
    let mut chunks = Vec::new();
    for event in events {
        chunks.push(format!("data: {event}\n\n"));
    }
    chunks.push("data: [DONE]\n\n".to_owned());
    chunks
}

fn sse_body() -> String {
    sse_events().join("")
}

fn step_body(stream: bool) -> crate::scripted::ScriptStep {
    if stream {
        return crate::scripted::script_chunks(200, &sse_events());
    }
    ok_text("hello")
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    #[test]
    fn t_models_handoff_9(steps in step_strategy()) {
        let mut script = vec![ok_text("later")];
        for step in steps.iter().rev() {
            script.push(step_body(step.stream));
        }
        let poster = ScriptPoster::new(script);
        let poster_seen = Arc::clone(&poster);
        let primary = model_ref(crate::prop_run::custom("m1", WireFormat::OpenAiCompatible));
        let other = model_ref(crate::prop_run::endpoint(
            ProviderKind::Custom,
            "http://127.0.0.1:9/v1",
            "",
            "m2",
        ));
        let armed = Arc::new(Mutex::new(Armed {
            handle: None,
            target: None,
            reason: SwitchReason::User,
        }));
        arm_hook(&poster, &armed);
        let mut session = session_with(poster);
        let mut serving = "m1".to_owned();
        let mut records = 0_usize;
        let request = hi();
        let mut parts = AskParts {
            session: &mut session,
            poster: &poster_seen,
            armed: &armed,
            primary: &primary,
            other: &other,
            request: &request,
        };
        for step in &steps {
            let outcome = ask_serving(&mut parts, step, &serving)?;
            records = records.saturating_add(outcome);
            if step.switch {
                serving = flip(&serving);
            }
        }
        let idle = Step {
            stream: false,
            switch: false,
            reason: 0,
        };
        let outcome = ask_serving(&mut parts, &idle, &serving)?;
        records = records.saturating_add(outcome);
        prop_assert!(records > 0 || steps.iter().all(|step| !step.switch));
    }
}

fn ask_serving(
    parts: &mut AskParts<'_>,
    step: &Step,
    serving: &str,
) -> Result<usize, TestCaseError> {
    let _: &str = serving;
    let mut plan = crate::session::ask_plan(
        parts.primary,
        std::slice::from_ref(parts.other),
        parts.request,
        Audience::Watching,
    );
    plan.stream = step.stream;
    let before = parts.poster.sent().len();
    prime(parts, step, serving);
    let outcome = match ask_one(parts.session, &mut plan) {
        Ok(outcome) => outcome,
        Err(err) => return Err(TestCaseError::fail(format!("{err:?}"))),
    };
    sent_model(parts.poster, before, &outcome.result.model)?;
    if step.stream {
        prop_assert!(sse_payloads(&sse_body()).len() > 1);
    }
    Ok(outcome.handoffs.len())
}

fn sent_model(poster: &ScriptPoster, before: usize, serving: &str) -> Result<(), TestCaseError> {
    let sent = poster.sent();
    let Some(row) = sent.get(before) else {
        return Err(TestCaseError::fail("sent"));
    };
    let needle = format!("\"model\":\"{serving}\"");
    prop_assert!(row.carries().contains(&needle));
    Ok(())
}

fn prime(parts: &AskParts<'_>, step: &Step, expected: &str) {
    let Ok(mut guard) = parts.armed.lock() else {
        return;
    };
    if step.switch {
        guard.handle = Some(parts.session.switch_handle());
        guard.reason = reason_of(step.reason);
        guard.target = Some(if expected == "m1" {
            copy_ref(parts.other)
        } else {
            copy_ref(parts.primary)
        });
    } else {
        guard.handle = None;
        guard.target = None;
    }
}

fn arm_hook(poster: &ScriptPoster, armed: &Arc<Mutex<Armed>>) {
    let armed = Arc::clone(armed);
    poster.arm_send(move || {
        let Ok(guard) = armed.lock() else {
            return;
        };
        let (Some(handle), Some(target)) = (guard.handle.as_ref(), guard.target.as_ref()) else {
            return;
        };
        handle.request(target, guard.reason);
    });
}

fn flip(expected: &str) -> String {
    if expected == "m1" {
        "m2".to_owned()
    } else {
        "m1".to_owned()
    }
}

fn copy_ref(model: &ModelRef) -> ModelRef {
    crate::model_copy::copy_model(model)
}
