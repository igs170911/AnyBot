use std::time::Duration;

use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use serde_json::Value;

pub const CLIENT_VERSION: &str = "0.1.0";
const REQUEST_BUDGET: Duration = Duration::from_secs(20);

pub struct ServerApi {
    client: reqwest::Client,
    base: String,
}

#[derive(Default)]
pub struct Call<'a> {
    pub bearer: Option<&'a str>,
    pub forwarded: Option<&'a str>,
}

impl ServerApi {
    pub fn new(port: u16) -> Result<Self, String> {
        let client = reqwest::Client::builder()
            .timeout(REQUEST_BUDGET)
            .build()
            .map_err(|e| format!("build client failed: {e}"))?;
        Ok(Self {
            client,
            base: format!("http://127.0.0.1:{port}"),
        })
    }

    pub async fn get(&self, path: &str, call: &Call<'_>) -> Result<(u16, Value), String> {
        let builder = self.client.get(format!("{}{path}", self.base));
        send(decorate(builder, call)).await
    }

    pub async fn delete(&self, path: &str) -> Result<u16, String> {
        let builder = self.client.delete(format!("{}{path}", self.base));
        send(decorate(builder, &Call::default()))
            .await
            .map(|(status, _body)| status)
    }

    pub async fn post(
        &self,
        path: &str,
        body: &Value,
        call: &Call<'_>,
    ) -> Result<(u16, Value), String> {
        let key = fresh_key()?;
        let builder = self
            .client
            .post(format!("{}{path}", self.base))
            .header("idempotency-key", key)
            .header(CONTENT_TYPE, "application/json")
            .body(body.to_string());
        send(decorate(builder, call)).await
    }
}

fn decorate(builder: reqwest::RequestBuilder, call: &Call<'_>) -> reqwest::RequestBuilder {
    let mut builder = builder.header("anybot-client-version", CLIENT_VERSION);
    if let Some(bearer) = call.bearer {
        builder = builder.header(AUTHORIZATION, format!("Bearer {bearer}"));
    }
    if let Some(address) = call.forwarded {
        builder = builder.header("x-forwarded-for", address);
    }
    builder
}

async fn send(builder: reqwest::RequestBuilder) -> Result<(u16, Value), String> {
    let response = builder
        .send()
        .await
        .map_err(|e| format!("server request failed: {e}"))?;
    let status = response.status().as_u16();
    let text = response
        .text()
        .await
        .map_err(|e| format!("read server response failed: {e}"))?;
    let value = serde_json::from_str(&text).unwrap_or(Value::String(text));
    Ok((status, value))
}

fn fresh_key() -> Result<String, String> {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).map_err(|e| format!("draw idempotency key failed: {e}"))?;
    Ok(u128::from_be_bytes(bytes).to_string())
}
