use rustls as _;
use std::time::Duration;
use trybuild as _;

use anybot_foundation as _;
use anybot_models::{
    Audience, DataPolicy, FailKind, ProviderKind, Role, SwitchReason, ThinkingMode, ask_plan,
};
use bytes as _;
use futures_util as _;
use reqwest as _;
use serde as _;
use serde_json as _;
use test_harness::{
    ModelFormat, arm_model_hold, model_hold_then_finish, model_replies_text, model_thinking_block,
    timeout_real,
};
use tokio as _;
use url as _;

#[path = "../test_support/drive.rs"]
mod drive;
#[path = "../test_support/session.rs"]
mod support;

#[tokio::test]
async fn t_models_handoff_1() -> Result<(), String> {
    let (world, pair) = support::pair(ModelFormat::OpenAi).await?;
    let release = arm_model_hold();
    support::script(
        &world.model,
        vec![model_hold_then_finish("partial-answer", true)],
    );
    support::script(&pair.other, vec![model_replies_text("next", false)]);
    let primary = pair.primary;
    let fallback = pair.fallback;
    let switched = anybot_models::ModelRef {
        endpoint: anybot_models::resolve_endpoint(
            ProviderKind::OpenAi,
            &fallback.endpoint.base_url,
            "k",
            "m2",
            Some(anybot_models::WireFormat::OpenAiCompatible),
        ),
        bot_context: None,
    };
    let handle_session = support::session(&world)?;
    let handle = handle_session.switch_handle();
    let other = std::sync::Arc::clone(&pair.other);
    let task = tokio::spawn(async move {
        let mut session = handle_session;
        let request = support::hi(ThinkingMode::Auto);
        let mut plan = ask_plan(
            &primary,
            std::slice::from_ref(&fallback),
            &request,
            Audience::Watching,
        );
        plan.stream = true;
        let first = session
            .ask(&mut plan)
            .await
            .map_err(|err| format!("{err:?}"));
        let during = other.recorded().len();
        let second = support::ask_watching(
            &mut session,
            &primary,
            std::slice::from_ref(&fallback),
            ThinkingMode::Auto,
        )
        .await;
        let rows = second
            .as_ref()
            .map(|row| row.handoffs.clone())
            .unwrap_or_default();
        (first, second, rows, during)
    });
    test_harness::sleep_real(Duration::from_millis(80)).await;
    handle.request(&switched, SwitchReason::User);
    let _sent = release.send(true);
    finish_handoff_1(&world, &pair.other, task).await
}

type HandoffJoin = tokio::task::JoinHandle<(
    Result<anybot_models::AskOutcome, String>,
    Result<anybot_models::AskOutcome, String>,
    Vec<anybot_models::HandoffRecord>,
    usize,
)>;

async fn finish_handoff_1(
    world: &support::World,
    other: &test_harness::FakeModel,
    task: HandoffJoin,
) -> Result<(), String> {
    let ready = timeout_real(
        drive::drive_until(&world.clock, || task.is_finished()),
        Duration::from_secs(8),
    )
    .await;
    assert!(ready.is_ok());
    let (first, second, handoffs, during) = task.await.map_err(|err| format!("{err}"))?;
    let first = first?;
    assert_eq!(first.result.model, "m1");
    assert!(first.result.text.contains("partial-answer"));
    assert_eq!(during, 0);
    assert_eq!(world.model.recorded().len(), 1);
    let second = second?;
    assert_eq!(second.result.model, "m2");
    assert_eq!(other.recorded().len(), 1);
    let row = handoffs
        .iter()
        .find(|row| row.after_model == "m2")
        .ok_or("missing handoff")?;
    assert_eq!(row.before_model, "m1");
    assert_eq!(row.reason, SwitchReason::User);
    assert_eq!(row.reason_code, "model-handoff-user");
    Ok(())
}

async fn ask_switched(
    claude: &support::World,
    openai: &support::World,
    attachment: anybot_models::NativeAttachment,
) -> Result<anybot_models::AskOutcome, String> {
    support::script(&openai.model, vec![model_replies_text("next", false)]);
    let next = anybot_models::ModelRef {
        endpoint: support::endpoint(ProviderKind::OpenAi, openai.port, "gpt-5", "k"),
        bot_context: None,
    };
    let mut session = anybot_models::Session::new(
        anybot_models::gateway(
            support::share(&claude.clock),
            &anybot_foundation::TrustedCerts::empty(),
        )?,
        DataPolicy::LocalOnly,
        support::share(&claude.clock),
        0,
    );
    let primary_first = anybot_models::ModelRef {
        endpoint: support::endpoint(ProviderKind::Anthropic, claude.port, "claude-opus-5-5", "k"),
        bot_context: None,
    };
    let _seed =
        support::ask_watching(&mut session, &primary_first, &[], ThinkingMode::Auto).await?;
    let request = kept_request(attachment);
    session.request_switch(&next, SwitchReason::User);
    let primary = anybot_models::ModelRef {
        endpoint: support::endpoint(ProviderKind::Anthropic, claude.port, "claude-opus-5-5", "k"),
        bot_context: None,
    };
    session
        .ask(&mut ask_plan(&primary, &[], &request, Audience::Watching))
        .await
        .map_err(|err| format!("{err:?}"))
}

fn kept_request(attachment: anybot_models::NativeAttachment) -> anybot_models::TurnRequest {
    let mut request = support::hi(ThinkingMode::Auto);
    request.messages.insert(
        0,
        anybot_models::Message {
            role: Role::Assistant,
            text: "kept-text".to_owned(),
            tool_calls: Vec::new(),
            tool_call_id: None,
        },
    );
    request.attachments.push(attachment);
    request
}

#[tokio::test]
async fn t_models_handoff_6() -> Result<(), String> {
    let claude = support::world(ModelFormat::Anthropic).await?;
    support::script(
        &claude.model,
        vec![
            model_thinking_block("plan", "sig-keep", "kept-text", false),
            model_replies_text("seed", false),
        ],
    );
    let endpoint = support::endpoint(ProviderKind::Anthropic, claude.port, "claude-opus-5-5", "k");
    let permit = anybot_models::SendPermit::for_endpoint(
        &DataPolicy::Cloud {
            allowed: vec![ProviderKind::Anthropic],
        },
        &endpoint,
        anybot_models::ModelUse::Bot,
    )
    .ok_or("permit")?;
    let first = anybot_models::gateway(
        support::share(&claude.clock),
        &anybot_foundation::TrustedCerts::empty(),
    )?
    .complete(&endpoint, &support::hi(ThinkingMode::Auto), false, &permit)
    .await
    .map_err(|err| format!("{err:?}"))?;
    let attachment = first.attachment.ok_or("missing attachment")?;
    let openai = support::world(ModelFormat::OpenAi).await?;
    let outcome = ask_switched(&claude, &openai, attachment).await?;
    assert_eq!(outcome.result.model, "gpt-5");
    assert!(!claude.model.recorded().is_empty());
    let body = openai
        .model
        .recorded()
        .first()
        .map(|row| row.body.clone())
        .ok_or("missing body")?;
    assert!(body.contains("kept-text"));
    assert!(!body.contains("sig-keep"));
    assert!(!body.contains("thinking"));
    assert!(!outcome.handoffs.is_empty());
    assert!(
        outcome
            .handoffs
            .iter()
            .any(|row| !row.reason_code.is_empty())
    );
    Ok(())
}

#[tokio::test]
async fn t_models_handoff_10() -> Result<(), String> {
    let world = support::world(ModelFormat::OpenAi).await?;
    let primary = anybot_models::ModelRef {
        endpoint: support::endpoint(ProviderKind::OpenAi, world.port, "m1", "k"),
        bot_context: None,
    };
    let next = anybot_models::ModelRef {
        endpoint: support::endpoint(ProviderKind::OpenAi, world.port, "m2", "k"),
        bot_context: None,
    };
    let mut session = support::session(&world)?;
    let key = anybot_models::model_key(ProviderKind::OpenAi, &next.endpoint.base_url, "m2");
    let mut learned = anybot_models::Learned::new(&key);
    learned.tools = Some(false);
    session.load_learned(vec![learned]);
    session.request_switch(&next, SwitchReason::User);
    let mut request = support::hi(ThinkingMode::Auto);
    request.tools.push(anybot_models::ToolSpec {
        name: "read_file".to_owned(),
        description: "read".to_owned(),
        parameters: serde_json::json!({}),
    });
    let err = session
        .ask(&mut ask_plan(&primary, &[], &request, Audience::Watching))
        .await
        .err()
        .ok_or("tools still sent")?;
    let anybot_models::GatewayFail::Classified { classified, .. } = err else {
        return Err("expected classified".to_owned());
    };
    assert_eq!(classified.kind, FailKind::NoTools);
    assert_eq!(classified.code, "model-no-tools");
    assert!(world.model.recorded().is_empty());
    Ok(())
}
