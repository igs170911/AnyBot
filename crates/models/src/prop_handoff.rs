use std::sync::{Arc, Mutex};

use proptest::prelude::*;

use crate::handoff::SwitchReason;
use crate::prop_run::{ask_one, hi, model_ref};
use crate::scripted::{ScriptPoster, ok_text, session_with};
use crate::session::{ModelRef, SwitchHandle};
use crate::types::Audience;

#[derive(Clone, Debug)]
struct Step {
    stream: bool,
    switch: bool,
    mid: bool,
    fail: bool,
    again: bool,
    reason: u8,
}

struct Armed {
    handle: Option<SwitchHandle>,
    target: Option<ModelRef>,
    reason: SwitchReason,
    mid: bool,
    again: bool,
}

struct AskParts<'a> {
    session: &'a mut crate::session::Session,
    poster: &'a ScriptPoster,
    armed: &'a Arc<Mutex<Armed>>,
    primary: &'a ModelRef,
    other: &'a ModelRef,
    spare: &'a ModelRef,
    request: &'a crate::types::TurnRequest,
}

fn step_strategy() -> impl Strategy<Value = Vec<Step>> {
    proptest::collection::vec(
        (
            any::<bool>(),
            any::<bool>(),
            any::<bool>(),
            any::<bool>(),
            any::<bool>(),
            0_u8..3,
        )
            .prop_map(|(stream, switch, mid, fail, again, reason)| Step {
                stream,
                switch,
                mid,
                fail,
                again,
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

fn step_body(step: &Step) -> crate::scripted::ScriptStep {
    if step.fail {
        return crate::scripted::http_fail(404, "missing");
    }
    let mut chunks = sse_events();
    if step.mid && chunks.len() > 1 {
        chunks.insert(
            1,
            "data: {\"choices\":[{\"delta\":{\"content\":\"mid\"}}]}\n\n".to_owned(),
        );
    }
    if step.stream || step.mid || step.switch {
        return crate::scripted::script_chunks(200, &chunks);
    }
    ok_text("hello")
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    #[test]
    fn t_models_handoff_9(steps in step_strategy()) {
        let mut script = vec![ok_text("later")];
        for step in steps.iter().rev() {
            script.push(step_body(step));
        }
        let poster = ScriptPoster::new(script);
        let poster_seen = Arc::clone(&poster);
        let primary = model_ref(named("m1"));
        let other = model_ref(named("m2"));
        let spare = model_ref(named("m3"));
        let armed = Arc::new(Mutex::new(Armed {
            handle: None,
            target: None,
            reason: SwitchReason::User,
            mid: false,
            again: false,
        }));
        arm_mid(&poster, &armed);
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
            spare: &spare,
            request: &request,
        };
        let mut carry = 0_usize;
        for step in &steps {
            let started = serving.clone();
            let outcome = ask_serving(&mut parts, step, &started)?;
            records = records.saturating_add(carry);
            carry = outcome;
            if step.switch {
                serving = flip(&serving);
            }
        }
        let idle = Step {
            stream: false,
            switch: false,
            mid: false,
            fail: false,
            again: false,
            reason: 0,
        };
        let outcome = ask_serving(&mut parts, &idle, &serving)?;
        records = records.saturating_add(carry).saturating_add(outcome);
        let switches = steps.iter().filter(|step| step.switch).count();
        prop_assert_eq!(records, switches);
    }
}

fn ask_serving(
    parts: &mut AskParts<'_>,
    step: &Step,
    serving: &str,
) -> Result<usize, TestCaseError> {
    let expected = serving.to_owned();
    let fallbacks = [copy_ref(parts.other), copy_ref(parts.spare)];
    let mut plan =
        crate::session::ask_plan(parts.primary, &fallbacks, parts.request, Audience::Watching);
    plan.stream = step.stream || step.mid || step.switch;
    let before = parts.poster.sent().len();
    prime(parts, step, &expected);
    let failed = ask_one(parts.session, &mut plan);
    let outcome = match failed {
        Ok(outcome) => outcome,
        Err(err) => {
            if step.fail {
                sent_model(parts.poster, before, &expected)?;
                prop_assert!(parts.poster.sent().len() > before);
                if step.switch {
                    fire_pending(parts.armed);
                }
                return Ok(0);
            }
            return Err(TestCaseError::fail(format!("{err:?}")));
        }
    };
    sent_model(parts.poster, before, &expected)?;
    one_model(parts.poster, before, &expected)?;
    if step.stream || step.mid || step.switch {
        let text = if step.mid { "helmidlo" } else { "hello" };
        prop_assert_eq!(&outcome.result.text, text);
    }
    Ok(outcome.handoffs.len())
}

fn one_model(poster: &ScriptPoster, before: usize, serving: &str) -> Result<(), TestCaseError> {
    let sent = poster.sent();
    let rows = sent.get(before..).unwrap_or(&[]);
    prop_assert!(!rows.is_empty());
    for row in rows {
        prop_assert!(row.shows_destination().contains(serving));
    }
    Ok(())
}

fn sent_model(poster: &ScriptPoster, before: usize, serving: &str) -> Result<(), TestCaseError> {
    let sent = poster.sent();
    let Some(row) = sent.get(before) else {
        return Err(TestCaseError::fail("sent"));
    };
    let host = row.shows_destination();
    if !host.contains(serving) {
        return Err(TestCaseError::fail(format!("saw {host} wanted {serving}")));
    }
    Ok(())
}

fn prime(parts: &AskParts<'_>, step: &Step, expected: &str) {
    let Ok(mut guard) = parts.armed.lock() else {
        return;
    };
    guard.mid = step.mid;
    guard.again = step.again;
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

fn arm_mid(poster: &ScriptPoster, armed: &Arc<Mutex<Armed>>) {
    let armed = Arc::clone(armed);
    poster.arm_sent(move || fire_switch(&armed, true));
}

fn fire_pending(armed: &Arc<Mutex<Armed>>) {
    fire_switch(armed, true);
}

fn fire_switch(armed: &Mutex<Armed>, during_send: bool) {
    let Ok(guard) = armed.lock() else {
        return;
    };
    let (Some(handle), Some(target)) = (guard.handle.as_ref(), guard.target.as_ref()) else {
        return;
    };
    if during_send {
        handle.request(target, guard.reason);
    }
    if guard.again {
        handle.request(target, guard.reason);
    }
}

fn named(model: &str) -> crate::types::ProviderEndpoint {
    crate::prop_run::endpoint(
        crate::types::ProviderKind::Custom,
        &format!("http://{model}.example/v1"),
        "",
        model,
    )
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
