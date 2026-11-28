use std::time::Duration;

use serde_json::{Value, json};

use crate::computers_support::computer_login;
use anybot_app_test::ClockWalker;
use anybot_app_test::http::{RequestSpec, ResponseSpec, post};
use anybot_app_test::mailbox_prop::make_bot;
use anybot_app_test::serve::{Served, boot_pair, serve_start};
use anybot_app_test::sse::{SseOpen, open_sse};

pub const BOT: &str = "bot-typing";
pub const SETTLE: Duration = Duration::from_millis(300);
pub const QUIET: Duration = Duration::from_millis(500);

pub struct Stage {
    pub served: Served,
    pub phone: String,
    pub computer: String,
    pub bob: String,
}

async fn bearer_of(
    served: &Served,
    account: &str,
    password: &str,
    key: &str,
) -> Result<String, String> {
    let body = json!({
        "account": account,
        "password": password,
        "label": "stage",
        "credential": "bearer",
    });
    let mut walker = ClockWalker::start(&served.clock);
    let answer = post(
        &served.base_url(),
        "/v1/account/login",
        &RequestSpec::new(&body.to_string()).with_key(key),
    )
    .await;
    walker.stop().await?;
    answer?.bearer()
}

pub async fn stage(name: &str) -> Result<Stage, String> {
    let served = serve_start(name).await?;
    boot_pair(&served).await?;
    let phone = bearer_of(&served, "alice", "alicepassword", "k-phone").await?;
    let bob = bearer_of(&served, "bob", "bobpassword", "k-bob-login").await?;
    let computer = computer_login(&served, "A", "k-a").await?.bearer;
    make_bot(&served, &phone, BOT, "k-bot").await?;
    Ok(Stage {
        served,
        phone,
        computer,
        bob,
    })
}

#[must_use]
pub fn typing_body(bot: &str, task: &str, text: &str) -> String {
    json!({
        "stream": {"bot": {"id": bot}},
        "task_id": task,
        "text": text,
        "thinking": "",
    })
    .to_string()
}

pub async fn send_typing(stage: &Stage, bearer: &str, body: &str) -> Result<ResponseSpec, String> {
    post(
        &stage.served.base_url(),
        "/v1/streams/typing",
        &RequestSpec::new(body).with_bearer(bearer),
    )
    .await
}

pub async fn watch(stage: &Stage, bearer: &str) -> Result<SseOpen, String> {
    let mut stream = open_sse(
        &stage.served.base_url(),
        "/v1/streams/notifications",
        bearer,
        "",
    )
    .await?;
    stream.recv_json_of("progress").await?;
    Ok(stream)
}

#[must_use]
pub fn text_of(hint: &Value) -> Option<&str> {
    hint.get("text").and_then(Value::as_str)
}
