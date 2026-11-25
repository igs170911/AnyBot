use std::net::IpAddr;
use std::time::Duration;

use reqwest::Client;
use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderName, HeaderValue};
use serde_json::Value;

pub const IDEMPOTENCY_HEADER: &str = "idempotency-key";
pub const PROXY_SECRET_HEADER: &str = "x-anybot-proxy-secret";
pub const FORWARDED_HEADER: &str = "x-forwarded-for";
pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(20);

pub struct RequestSpec {
    pub key: Option<String>,
    pub bearer: Option<String>,
    pub body: String,
    pub extra: Vec<(String, String)>,
    pub local: Option<IpAddr>,
}

impl RequestSpec {
    #[must_use]
    pub fn new(body: &str) -> Self {
        RequestSpec {
            key: None,
            bearer: None,
            body: body.to_owned(),
            extra: Vec::new(),
            local: None,
        }
    }

    #[must_use]
    pub fn with_local(mut self, local: IpAddr) -> Self {
        self.local = Some(local);
        self
    }

    #[must_use]
    pub fn with_key(mut self, key: &str) -> Self {
        self.key = Some(key.to_owned());
        self
    }

    #[must_use]
    pub fn with_bearer(mut self, bearer: &str) -> Self {
        self.bearer = Some(bearer.to_owned());
        self
    }

    #[must_use]
    pub fn with_header(mut self, name: &str, value: &str) -> Self {
        self.extra.push((name.to_owned(), value.to_owned()));
        self
    }

    fn headers(&self) -> Result<HeaderMap, String> {
        let mut map = HeaderMap::new();
        map.insert(
            reqwest::header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );
        if let Some(key) = &self.key {
            let value = HeaderValue::from_str(key).map_err(|e| e.to_string())?;
            map.insert(HeaderName::from_static(IDEMPOTENCY_HEADER), value);
        }
        if let Some(bearer) = &self.bearer {
            let value =
                HeaderValue::from_str(&format!("Bearer {bearer}")).map_err(|e| e.to_string())?;
            map.insert(AUTHORIZATION, value);
        }
        for (name, value) in &self.extra {
            let parsed: HeaderName = name
                .parse()
                .map_err(|e: reqwest::header::InvalidHeaderName| e.to_string())?;
            let header = HeaderValue::from_str(value).map_err(|e| e.to_string())?;
            map.insert(parsed, header);
        }
        Ok(map)
    }
}

pub struct ResponseSpec {
    pub status: u16,
    pub json: Value,
    pub elapsed: Duration,
}

impl ResponseSpec {
    pub fn field_str(&self, key: &str) -> Result<String, String> {
        self.json
            .get(key)
            .and_then(Value::as_str)
            .map(String::from)
            .ok_or(format!("no string field {key} in {}", self.json))
    }

    pub fn bearer(&self) -> Result<String, String> {
        self.field_str("session_key")
    }

    #[must_use]
    pub fn code(&self) -> Option<String> {
        self.json
            .get("code")
            .and_then(Value::as_str)
            .map(String::from)
    }
}

pub async fn post(base: &str, path: &str, spec: &RequestSpec) -> Result<ResponseSpec, String> {
    let client = build_client(spec)?;
    send(&client, base, path, spec, "POST").await
}

pub async fn get(base: &str, path: &str, spec: &RequestSpec) -> Result<ResponseSpec, String> {
    let client = build_client(spec)?;
    send(&client, base, path, spec, "GET").await
}

pub struct ConnectionPool {
    client: Client,
}

impl ConnectionPool {
    pub fn new(pool: usize) -> Result<Self, String> {
        let client = Client::builder()
            .pool_max_idle_per_host(pool)
            .build()
            .map_err(|e| e.to_string())?;
        Ok(ConnectionPool { client })
    }

    pub async fn get(
        &self,
        base: &str,
        path: &str,
        spec: &RequestSpec,
    ) -> Result<ResponseSpec, String> {
        send(&self.client, base, path, spec, "GET").await
    }

    pub async fn post(
        &self,
        base: &str,
        path: &str,
        spec: &RequestSpec,
    ) -> Result<ResponseSpec, String> {
        send(&self.client, base, path, spec, "POST").await
    }
}

fn build_client(spec: &RequestSpec) -> Result<Client, String> {
    match spec.local {
        None => Ok(Client::new()),
        Some(local) => Client::builder()
            .local_address(local)
            .build()
            .map_err(|e| e.to_string()),
    }
}

async fn send(
    client: &Client,
    base: &str,
    path: &str,
    spec: &RequestSpec,
    method: &str,
) -> Result<ResponseSpec, String> {
    let url = format!("{base}{path}");
    let mut request = match method {
        "POST" => client.post(url),
        _ => client.get(url),
    };
    request = request.headers(spec.headers()?);
    request = request.timeout(REQUEST_TIMEOUT);
    if method == "POST" {
        request = request.body(spec.body.clone());
    }
    let started = std::time::Instant::now();
    let response = request.send().await.map_err(|e| e.to_string())?;
    let elapsed = started.elapsed();
    let status = response.status().as_u16();
    let text = response.text().await.map_err(|e| e.to_string())?;
    let json = if text.is_empty() {
        Value::Null
    } else {
        serde_json::from_str::<Value>(&text).map_err(|e| e.to_string())?
    };
    Ok(ResponseSpec {
        status,
        json,
        elapsed,
    })
}
