use reqwest::Client;

use crate::profile::LiveProfile;

pub fn live_client(profile: &LiveProfile) -> Result<Client, String> {
    let trusted = anybot_foundation::TrustedCerts::parse_pem(&profile.ca_pem)?;
    let store = anybot_foundation::TrustStore::new(&trusted)?;
    let tls = store.client_config()?;
    Client::builder()
        .use_preconfigured_tls(tls)
        .redirect(reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| format!("build HTTPS client failed: {e}"))
}

#[must_use]
pub fn chat_url(base_url: &str) -> String {
    base_url.trim_end_matches('/').to_owned() + "/chat/completions"
}

#[must_use]
pub fn normal_request(model: &str) -> String {
    request_body(model, false, &plain_messages())
}

#[must_use]
pub fn stream_request(model: &str) -> String {
    request_body(model, true, &plain_messages())
}

#[must_use]
pub fn tool_request(model: &str) -> String {
    let forced = serde_json::json!({ "type": "function", "function": { "name": TOOL_NAME } });
    let body = serde_json::json!({
        "model": model,
        "stream": false,
        "messages": [
            { "role": "system", "content": TOOL_SYSTEM },
            { "role": "user", "content": FORMAT_PROBE_PROMPT },
        ],
        "tools": [tool_schema()],
        "tool_choice": forced,
    });
    body.to_string()
}

#[must_use]
pub fn tool_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "function",
        "function": {
            "name": TOOL_NAME,
            "description": "Answer with the fixed word OK",
            "parameters": {
                "type": "object",
                "properties": {
                    "word": { "type": "string", "description": "The fixed answer word" }
                },
                "required": ["word"],
                "additionalProperties": false
            },
        },
    })
}

#[must_use]
pub fn tool_arguments() -> serde_json::Value {
    serde_json::json!({ "word": "OK" })
}

pub const TOOL_NAME: &str = "probe_tool";
pub const TOOL_SYSTEM: &str = "Always answer by calling the probe_tool function. Pass word as OK.";
pub const FORMAT_PROBE_PROMPT: &str = "Reply with the single word probe.";

#[must_use]
fn plain_messages() -> serde_json::Value {
    serde_json::json!([{ "role": "user", "content": FORMAT_PROBE_PROMPT }])
}

fn request_body(model: &str, stream: bool, messages: &serde_json::Value) -> String {
    let body = serde_json::json!({
        "model": model,
        "stream": stream,
        "messages": messages,
    });
    body.to_string()
}
