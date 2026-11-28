use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use test_harness::server_client::{Call, ServerApi};

pub const ADMIN_ACCOUNT: &str = "alice";

#[derive(Clone)]
pub struct Admin {
    pub account: String,
    pub password: String,
    pub bearer: String,
}

#[derive(Deserialize)]
pub struct SeedRequest {
    pub bots: Vec<String>,
    #[serde(default)]
    pub admin: bool,
}

#[derive(Serialize)]
pub struct SeededBot {
    pub name: String,
    pub id: String,
    pub model: String,
}

#[derive(Serialize)]
pub struct SeedReply {
    pub account: String,
    pub password: String,
    pub bots: Vec<SeededBot>,
}

static ADDRESS_COUNTER: AtomicU64 = AtomicU64::new(1);

fn fresh_address() -> String {
    let n = ADDRESS_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("10.{}.{}.{}", (n >> 16) & 255, (n >> 8) & 255, n & 255)
}

fn expect_status(label: &str, got: (u16, Value), want: u16) -> Result<Value, String> {
    if got.0 == want {
        return Ok(got.1);
    }
    Err(format!(
        "{label} answered {} instead of {want}: {}",
        got.0, got.1
    ))
}

fn text_field(value: &Value, key: &str) -> Result<String, String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| format!("reply has no text field {key}: {value}"))
}

pub async fn register_and_login(
    api: &ServerApi,
    account: &str,
    password: &str,
    invite: &str,
) -> Result<String, String> {
    let address = fresh_address();
    let call = Call {
        forwarded: Some(&address),
        ..Call::default()
    };
    let body = json!({"account": account, "password": password, "invite": invite});
    expect_status(
        "register",
        api.post("/v1/account/register", &body, &call).await?,
        201,
    )?;
    login_bearer(api, account, password).await
}

pub async fn login_bearer(
    api: &ServerApi,
    account: &str,
    password: &str,
) -> Result<String, String> {
    let body = json!({
        "account": account,
        "password": password,
        "label": "testenv",
        "credential": "bearer",
    });
    let reply = api
        .post("/v1/account/login", &body, &Call::default())
        .await?;
    text_field(&expect_status("login", reply, 200)?, "bearer_key")
}

#[must_use]
pub fn random_name(prefix: &str) -> String {
    let id = uuid::Uuid::now_v7().simple().to_string();
    let tail: String = id.chars().rev().take(10).collect();
    format!("{prefix}{tail}")
}

pub async fn read_invite(api: &ServerApi, admin: &Admin) -> Result<String, String> {
    let call = Call {
        bearer: Some(&admin.bearer),
        ..Call::default()
    };
    let reply = expect_status("invite", api.get("/v1/admin/invite", &call).await?, 200)?;
    text_field(&reply, "invite")
}

pub async fn create_bots(
    api: &ServerApi,
    bearer: &str,
    names: &[String],
    now_ms: i64,
) -> Result<Vec<SeededBot>, String> {
    let mut bots = Vec::new();
    let mut items = Vec::new();
    for name in names {
        let id = uuid::Uuid::now_v7().to_string();
        let model = format!("fake-{}", uuid::Uuid::now_v7());
        items.push(json!({
            "device_id": uuid::Uuid::now_v7().to_string(),
            "kind": "bots_create",
            "content": {"bot_id": id, "name": name, "model": model},
            "created_at_ms": now_ms,
            "offline_sent": false,
        }));
        bots.push(SeededBot {
            name: name.clone(),
            id,
            model,
        });
    }
    let call = Call {
        bearer: Some(bearer),
        ..Call::default()
    };
    let body = json!({"stream": "account", "items": items});
    let reply = expect_status(
        "bots_create",
        api.post("/v1/streams/append", &body, &call).await?,
        200,
    )?;
    reject_unaccepted(&reply)?;
    Ok(bots)
}

fn reject_unaccepted(reply: &Value) -> Result<(), String> {
    let results = reply
        .get("results")
        .and_then(Value::as_array)
        .ok_or_else(|| format!("append reply has no results: {reply}"))?;
    let all_accepted = results
        .iter()
        .all(|row| row.get("result").and_then(Value::as_str) == Some("accepted"));
    if all_accepted {
        return Ok(());
    }
    Err(format!("append did not accept every bot: {reply}"))
}

pub async fn seed_world(
    api: &ServerApi,
    admin: &Admin,
    request: &SeedRequest,
    now_ms: i64,
) -> Result<SeedReply, String> {
    let (account, password, bearer) = if request.admin {
        (
            admin.account.clone(),
            admin.password.clone(),
            admin.bearer.clone(),
        )
    } else {
        let invite = read_invite(api, admin).await?;
        let account = random_name("u");
        let password = format!("pw-{}", uuid::Uuid::now_v7());
        let bearer = register_and_login(api, &account, &password, &invite).await?;
        (account, password, bearer)
    };
    let bots = create_bots(api, &bearer, &request.bots, now_ms).await?;
    Ok(SeedReply {
        account,
        password,
        bots,
    })
}

pub async fn disable_account(api: &ServerApi, admin: &Admin, account: &str) -> Result<(), String> {
    let call = Call {
        bearer: Some(&admin.bearer),
        ..Call::default()
    };
    let path = format!("/v1/admin/accounts/{account}/disable");
    let reply = api.post(&path, &json!({"ok": true}), &call).await?;
    expect_status("disable account", reply, 200).map(drop)
}

pub async fn bootstrap_admin(api: &ServerApi, pool: &anybot_server::Pool) -> Result<Admin, String> {
    let invite = anybot_server::draw_invite()?;
    anybot_server::write_config_row(pool, anybot_server::KEY_INVITE_CODE, &invite)
        .await
        .map_err(|e| format!("write invite failed: {e}"))?;
    let password = format!("pw-{}", uuid::Uuid::now_v7());
    let bearer = register_and_login(api, ADMIN_ACCOUNT, &password, &invite).await?;
    Ok(Admin {
        account: ADMIN_ACCOUNT.to_owned(),
        password,
        bearer,
    })
}
