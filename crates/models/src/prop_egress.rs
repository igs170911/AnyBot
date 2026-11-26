use std::sync::Arc;

use proptest::prelude::*;

use crate::egress::{DataPolicy, JudgeCandidate, ModelUse, pick_judge};
use crate::prop_run::{ask_one, hi, model_ref, poster_of};
use crate::scripted::{ScriptStep, connect_fail, http_fail, ok_text};
use crate::session::ModelRef;
use crate::types::{Audience, ProviderKind};
use crate::where_run::RunPlace;

struct PlaceRule {
    loopback: bool,
    private_net: bool,
    bare_name: bool,
    dotted: bool,
}

#[derive(Clone, Debug)]
struct BotCase {
    primary: u8,
    fallbacks: Vec<u8>,
    local_only: bool,
    allow_openai: bool,
    use_kind: u8,
    redirect: bool,
    fail: u8,
    judges: Vec<u8>,
}

fn bot_strategy() -> impl Strategy<Value = BotCase> {
    (
        0_u8..5,
        proptest::collection::vec(0_u8..5, 0..9),
        any::<bool>(),
        any::<bool>(),
        0_u8..6,
        any::<bool>(),
        0_u8..5,
        proptest::collection::vec(0_u8..4, 0..4),
    )
        .prop_map(
            |(primary, fallbacks, local_only, allow_openai, use_kind, redirect, fail, judges)| {
                BotCase {
                    primary,
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

fn place_model(code: u8, name: &str) -> ModelRef {
    let (kind, url, key) = match code % 5 {
        0 => (ProviderKind::Custom, "http://127.0.0.1:9/v1", ""),
        1 => (ProviderKind::Custom, "http://192.168.1.20:9/v1", ""),
        2 => (ProviderKind::Custom, "http://gpu:8000/v1", ""),
        3 => (ProviderKind::OpenAi, "https://api.openai.com/v1", "k"),
        _ => (ProviderKind::Anthropic, "https://api.anthropic.com", "k"),
    };
    model_ref(crate::prop_run::endpoint(kind, url, key, name))
}

fn known_cloud(model: &ModelRef) -> bool {
    cloud_host(&model.endpoint.base_url)
}

fn cloud_host(raw: &str) -> bool {
    let host = host_of(raw);
    let rule = PlaceRule {
        loopback: host == "127.0.0.1" || host == "localhost" || host == "::1",
        private_net: host.starts_with("192.168.")
            || host.starts_with("10.")
            || host.starts_with("172.16."),
        bare_name: !host.contains('.') && !host.is_empty(),
        dotted: host.contains('.'),
    };
    rule.dotted && !rule.loopback && !rule.private_net && !rule.bare_name
}

fn host_of(raw: &str) -> String {
    let without = raw.split("://").nth(1).unwrap_or(raw);
    let host = without.split('/').next().unwrap_or(without);
    host.split(':').next().unwrap_or(host).to_owned()
}

fn fail_step(kind: u8, _redirect: bool) -> ScriptStep {
    match kind % 5 {
        0 => http_fail(500, "down"),
        1 => http_fail(401, "no"),
        2 => connect_fail(),
        3 => http_fail(429, "busy"),
        _ => crate::scripted::blank_step(),
    }
}

fn script_for(case: &BotCase) -> Vec<ScriptStep> {
    let mut steps = vec![ok_text("done")];
    for _ in 0..8 {
        steps.push(fail_step(case.fail, case.redirect));
    }
    steps
}

fn s8(kind: ModelUse) -> bool {
    matches!(kind, ModelUse::Summary | ModelUse::Embed | ModelUse::Rerank)
}

fn judge_ok(policy: &DataPolicy, picked: Option<&JudgeCandidate>) -> bool {
    let Some(picked) = picked else {
        return true;
    };
    if matches!(picked.place, RunPlace::Cloud) && matches!(policy, DataPolicy::LocalOnly) {
        return false;
    }
    picked.has_key
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    #[test]
    fn t_models_egress_6(case in bot_strategy()) {
        let policy = policy_of(&case);
        let kind = use_kind(case.use_kind);
        let primary = place_model(case.primary, "m0");
        let fallbacks: Vec<ModelRef> = case
            .fallbacks
            .iter()
            .enumerate()
            .map(|(index, code)| place_model(*code, &format!("f{index}")))
            .collect();
        let poster = poster_of(script_for(&case));
        let sent_poster = Arc::clone(&poster);
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
        for row in sent_poster.sent() {
            if case.local_only || s8(kind) {
                prop_assert!(!cloud_host(&row.url));
            }
            prop_assert!(!row.shows_destination().contains("api.cloud.test"));
        }
        if case.redirect && case.fail == 0 && case.fallbacks.is_empty() {
            prop_assert!(!live_redirect_followed());
        }
        assert_local_uses(&policy, &primary, &fallbacks, kind)?;
        let judges = judges_of(&case);
        let picked = pick_judge(&policy, ProviderKind::OpenAi, &judges);
        prop_assert!(judge_ok(&policy, picked));
    }
}

fn assert_local_uses(
    policy: &DataPolicy,
    primary: &ModelRef,
    fallbacks: &[ModelRef],
    kind: ModelUse,
) -> Result<(), TestCaseError> {
    if !s8(kind) && !matches!(policy, DataPolicy::LocalOnly) {
        return Ok(());
    }
    let mut models = vec![primary];
    models.extend(fallbacks);
    for model in models {
        if known_cloud(model) {
            prop_assert!(cloud_host(&model.endpoint.base_url));
        }
    }
    Ok(())
}

fn live_redirect_followed() -> bool {
    let followed = std::sync::Mutex::new(true);
    let ran = std::thread::scope(|scope| {
        let handle = scope.spawn(|| {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("runtime");
            runtime.block_on(redirect_once())
        });
        handle.join()
    });
    if let Ok(Ok(missed)) = ran
        && let Ok(mut slot) = followed.lock()
    {
        *slot = missed;
    }
    followed.lock().map_or(true, |slot| *slot)
}

async fn redirect_once() -> Result<bool, String> {
    let (local, local_port) =
        test_harness::model_start_with_format(test_harness::ModelFormat::OpenAi).await?;
    let (cloud, cloud_port) =
        test_harness::model_start_with_format(test_harness::ModelFormat::OpenAi).await?;
    local.script_set(vec![test_harness::model_redirect(&format!(
        "http://127.0.0.1:{cloud_port}/v1/chat/completions"
    ))]);
    let clock: Arc<dyn anybot_foundation::clock::Clock> =
        Arc::new(anybot_foundation::clock::SystemClock);
    let gateway = crate::gateway(clock, &anybot_foundation::TrustedCerts::empty())?;
    let endpoint = crate::prop_run::endpoint(
        ProviderKind::Custom,
        &format!("http://127.0.0.1:{local_port}/v1"),
        "",
        "local",
    );
    let permit =
        crate::egress::SendPermit::for_endpoint(&DataPolicy::LocalOnly, &endpoint, ModelUse::Bot)
            .ok_or_else(|| "permit".to_owned())?;
    let failed = gateway
        .complete(&endpoint, &hi(), false, &permit)
        .await
        .is_err();
    Ok(!failed || !cloud.recorded().is_empty())
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
            place: match code % 3 {
                0 => RunPlace::Local,
                1 => RunPlace::Lan,
                _ => RunPlace::Cloud,
            },
        })
        .collect()
}
