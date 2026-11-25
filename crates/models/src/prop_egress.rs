use std::sync::Arc;

use proptest::prelude::*;

use crate::egress::{DataPolicy, JudgeCandidate, ModelUse, SendPermit, allowed, pick_judge};
use crate::prop_run::{ask_one, hi, model_ref, poster_of};
use crate::scripted::{ScriptStep, connect_fail, http_fail, ok_text};
use crate::session::ModelRef;
use crate::types::{Audience, ProviderKind, WireFormat};
use crate::where_run::RunPlace;

#[derive(Clone, Debug)]
struct BotCase {
    local_primary: bool,
    fallbacks: Vec<bool>,
    local_only: bool,
    allow_openai: bool,
    use_kind: u8,
    redirect: bool,
    fail: u8,
    judges: Vec<u8>,
}

fn bot_strategy() -> impl Strategy<Value = BotCase> {
    (
        any::<bool>(),
        proptest::collection::vec(any::<bool>(), 0..9),
        any::<bool>(),
        any::<bool>(),
        0_u8..6,
        any::<bool>(),
        0_u8..4,
        proptest::collection::vec(0_u8..4, 0..4),
    )
        .prop_map(
            |(
                local_primary,
                fallbacks,
                local_only,
                allow_openai,
                use_kind,
                redirect,
                fail,
                judges,
            )| {
                BotCase {
                    local_primary,
                    fallbacks,
                    local_only,
                    allow_openai,
                    use_kind,
                    redirect,
                    fail,
                    judges,
                }
            },
        )
}

fn use_kind(code: u8) -> ModelUse {
    match code % 6 {
        0 => ModelUse::Bot,
        1 => ModelUse::Fallback,
        2 => ModelUse::Judge,
        3 => ModelUse::Summary,
        4 => ModelUse::Embed,
        _ => ModelUse::Rerank,
    }
}

fn policy_of(case: &BotCase) -> DataPolicy {
    if case.local_only {
        return DataPolicy::LocalOnly;
    }
    let mut allowed = Vec::new();
    if case.allow_openai {
        allowed.push(ProviderKind::OpenAi);
    }
    DataPolicy::Cloud { allowed }
}

fn place_model(local: bool, name: &str) -> ModelRef {
    if local {
        return model_ref(crate::prop_run::custom(name, WireFormat::OpenAiCompatible));
    }
    model_ref(crate::prop_run::endpoint(
        ProviderKind::OpenAi,
        "https://api.openai.com/v1",
        "k",
        name,
    ))
}

fn known_cloud(model: &ModelRef) -> bool {
    model.endpoint.base_url.contains("api.openai.com")
}

fn fail_step(kind: u8, redirect: bool) -> ScriptStep {
    if redirect && kind.is_multiple_of(2) {
        return crate::scripted::script_http(302, "http://api.cloud.test/v1/chat/completions");
    }
    match kind % 4 {
        0 => http_fail(500, "down"),
        1 => http_fail(401, "no"),
        2 => connect_fail(),
        _ => http_fail(429, "busy"),
    }
}

fn script_for(case: &BotCase) -> Vec<ScriptStep> {
    let mut steps = vec![ok_text("done")];
    for _ in 0..8 {
        steps.push(fail_step(case.fail, case.redirect));
    }
    steps
}

fn expected_place(url: &str) -> RunPlace {
    if url.contains("192.168.") || url.contains("127.0.0.1") {
        return RunPlace::Lan;
    }
    RunPlace::Cloud
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    #[test]
    fn t_models_egress_6(case in bot_strategy()) {
        let policy = policy_of(&case);
        let kind = use_kind(case.use_kind);
        let primary = place_model(case.local_primary, "m0");
        let fallbacks: Vec<ModelRef> = case
            .fallbacks
            .iter()
            .enumerate()
            .map(|(index, local)| place_model(*local, &format!("f{index}")))
            .collect();
        let poster = poster_of(script_for(&case));
        let sent_poster = Arc::clone(&poster);
        if matches!(kind, ModelUse::Bot | ModelUse::Fallback | ModelUse::Judge) {
            let mut session = crate::scripted::session_policy(poster, policy.clone());
            let request = hi();
            let mut plan = crate::session::ask_plan(
                &primary,
                &fallbacks,
                &request,
                Audience::Watching,
            );
            plan.use_kind = kind;
            let _outcome = ask_one(&mut session, &mut plan);
        }
        for row in sent_poster.sent() {
            if case.local_only {
                prop_assert!(!matches!(expected_place(&row.url), RunPlace::Cloud));
            }
            prop_assert!(!row.shows_destination().contains("api.cloud.test"));
        }
        assert_local_uses(&policy, &primary, &fallbacks, kind)?;
        let _picked = pick_judge(&policy, ProviderKind::OpenAi, &judges_of(&case));
    }
}

fn assert_local_uses(
    policy: &DataPolicy,
    primary: &ModelRef,
    fallbacks: &[ModelRef],
    kind: ModelUse,
) -> Result<(), TestCaseError> {
    if !matches!(kind, ModelUse::Summary | ModelUse::Embed | ModelUse::Rerank) {
        return Ok(());
    }
    let mut models = vec![primary];
    models.extend(fallbacks);
    for model in models {
        let issued = SendPermit::for_endpoint(policy, &model.endpoint, kind);
        if known_cloud(model) {
            prop_assert!(issued.is_none());
            prop_assert!(!allowed(
                policy,
                &model.endpoint.base_url,
                model.endpoint.kind,
                kind
            ));
        }
    }
    Ok(())
}

fn judges_of(case: &BotCase) -> Vec<JudgeCandidate> {
    case.judges
        .iter()
        .map(|code| JudgeCandidate {
            provider: match code % 4 {
                0 => ProviderKind::OpenAi,
                1 => ProviderKind::Anthropic,
                2 => ProviderKind::Ollama,
                _ => ProviderKind::Custom,
            },
            has_key: code % 2 == 0,
            place: if code % 3 == 0 {
                RunPlace::Cloud
            } else {
                RunPlace::Local
            },
        })
        .collect()
}
