use serde_json::{Value, json};

use anybot_app_test::ClockWalker;
use anybot_app_test::http::{RequestSpec, ResponseSpec, get, post};
use anybot_app_test::serve::Served;

pub struct ComputerLogin {
    pub bearer: String,
    pub device_id: String,
}

pub async fn login_as(
    served: &Served,
    label: &str,
    credential: &str,
    key: &str,
) -> Result<ResponseSpec, String> {
    let body = json!({
        "account": "alice",
        "password": "alicepassword",
        "label": label,
        "credential": credential,
    });
    let mut walker = ClockWalker::start(&served.clock);
    let answer = post(
        &served.base_url(),
        "/v1/account/login",
        &RequestSpec::new(&body.to_string()).with_key(key),
    )
    .await;
    walker.stop().await?;
    answer
}

pub async fn computer_login(
    served: &Served,
    label: &str,
    key: &str,
) -> Result<ComputerLogin, String> {
    let answer = login_as(served, label, "computer", key).await?;
    if answer.status != 200 {
        return Err(format!("computer login {label}: {}", answer.json));
    }
    Ok(ComputerLogin {
        bearer: answer.bearer()?,
        device_id: answer.field_str("device_id")?,
    })
}

pub async fn api_post(
    served: &Served,
    bearer: &str,
    path: &str,
    body: &Value,
    key: &str,
) -> Result<ResponseSpec, String> {
    let mut walker = ClockWalker::start(&served.clock);
    let answer = post(
        &served.base_url(),
        path,
        &RequestSpec::new(&body.to_string())
            .with_key(key)
            .with_bearer(bearer),
    )
    .await;
    walker.stop().await?;
    answer
}

pub async fn api_get(served: &Served, bearer: &str, path: &str) -> Result<ResponseSpec, String> {
    get(
        &served.base_url(),
        path,
        &RequestSpec::new("").with_bearer(bearer),
    )
    .await
}

pub async fn computer_rows(served: &Served, bearer: &str) -> Result<Vec<Value>, String> {
    let answer = api_get(served, bearer, "/v1/computers").await?;
    if answer.status != 200 {
        return Err(format!("list computers: {}", answer.json));
    }
    answer
        .json
        .get("computers")
        .and_then(Value::as_array)
        .cloned()
        .ok_or_else(|| format!("no computers array in {}", answer.json))
}

#[must_use]
pub fn row_of<'a>(rows: &'a [Value], device_id: &str) -> Option<&'a Value> {
    rows.iter()
        .find(|row| row.get("device_id").and_then(Value::as_str) == Some(device_id))
}

#[must_use]
pub fn name_of(row: &Value) -> Option<&str> {
    row.get("name").and_then(Value::as_str)
}
