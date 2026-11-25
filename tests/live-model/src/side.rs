use std::path::{Path, PathBuf};

use reqwest::{Client, Method};
use test_harness::{
    FakeModel, ModelFormat, model_start_with_format, plain_client, request, sse_post_client,
    sse_send,
};

use crate::cases::{ProbeCase, probe_cases};
use crate::probe::{chat_url, live_client};
use crate::product::NeutralShape;
use crate::profile::{LiveProfile, REPORT_REL, profile_dir};
use crate::shape::{json_shape, stream_shape};

pub struct Outcome<S> {
    pub side: S,
    pub failed: Option<String>,
}

impl<S> Outcome<S> {
    pub fn ok(side: S) -> Self {
        Self { side, failed: None }
    }

    pub fn partial(side: S, message: String) -> Self {
        Self {
            side,
            failed: Some(message),
        }
    }
}

pub struct FakeSide {
    pub probe: Vec<Vec<String>>,
    pub product: Vec<NeutralShape>,
}

pub struct LiveSide {
    pub probe: Vec<Vec<String>>,
    pub product: Vec<NeutralShape>,
}

pub async fn fake_side(model_name: &str) -> Outcome<FakeSide> {
    let started = match model_start_with_format(ModelFormat::Vllm).await {
        Ok(pair) => pair,
        Err(message) => {
            let side = FakeSide {
                probe: Vec::new(),
                product: Vec::new(),
            };
            return Outcome::partial(side, format!("fake model start failed: {message}"));
        }
    };
    let (model, port) = started;
    model.script_set(crate::cases::fake_script(model_name));
    let outcome = fake_side_requests(&model, port, model_name).await;
    drop(model);
    outcome
}

async fn fake_side_requests(_model: &FakeModel, port: u16, model_name: &str) -> Outcome<FakeSide> {
    let mut side = FakeSide {
        probe: Vec::new(),
        product: Vec::new(),
    };
    let url = format!("http://127.0.0.1:{port}/v1/chat/completions");
    let client = match plain_client() {
        Ok(client) => client,
        Err(message) => return Outcome::partial(side, message),
    };
    for case in probe_cases(model_name) {
        match probe_plain(&client, &url, &case).await {
            Ok(shape) => side.probe.push(shape),
            Err(message) => return Outcome::partial(side, message),
        }
    }
    let product = crate::product::fake_product_shapes(port, model_name).await;
    side.product = product.side;
    match product.failed {
        Some(message) => Outcome::partial(side, message),
        None => Outcome::ok(side),
    }
}

pub async fn live_side(profile: &LiveProfile) -> Outcome<LiveSide> {
    let client = match live_client(profile) {
        Ok(client) => client,
        Err(message) => {
            let side = LiveSide {
                probe: Vec::new(),
                product: Vec::new(),
            };
            return Outcome::partial(side, message);
        }
    };
    let mut side = LiveSide {
        probe: Vec::new(),
        product: Vec::new(),
    };
    let url = chat_url(&profile.base_url);
    for case in probe_cases(&profile.model) {
        match probe_authorized(&client, &url, &case, profile).await {
            Ok(shape) => side.probe.push(shape),
            Err(message) => return Outcome::partial(side, message),
        }
    }
    let product = crate::product::live_product_shapes(profile).await;
    side.product = product.side;
    match product.failed {
        Some(message) => Outcome::partial(side, message),
        None => Outcome::ok(side),
    }
}

pub fn write_report(repo: &Path, shapes: &Shapes) -> Result<PathBuf, String> {
    let mut lines = report_lines(shapes);
    lines.push(String::new());
    let text = lines.join("\n");
    let path = repo.join(REPORT_REL);
    std::fs::create_dir_all(profile_dir(repo)).map_err(|e| e.to_string())?;
    std::fs::write(&path, text).map_err(|e| format!("write report failed: {e}"))?;
    Ok(path)
}

#[must_use]
pub fn report_lines(shapes: &Shapes) -> Vec<String> {
    let mut out = Vec::new();
    for (index, name) in crate::cases::SCENARIOS.iter().copied().enumerate() {
        let fake = shapes.fake.get(index).cloned().unwrap_or_default();
        let live = shapes.live.get(index).cloned().unwrap_or_default();
        out.push(format!("scenario {name}:"));
        out.extend(diff_tokens(&fake, &live));
    }
    out
}

pub struct Shapes {
    pub fake: Vec<Vec<String>>,
    pub live: Vec<Vec<String>>,
}

#[must_use]
pub fn diff_tokens_public(fake: &[String], live: &[String]) -> Vec<String> {
    diff_tokens(fake, live)
}

fn diff_tokens(fake: &[String], live: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for token in unique_sorted(fake, live) {
        match (count_of(fake, &token) > 0, count_of(live, &token) > 0) {
            (true, true) | (false, false) => {}
            (true, false) => out.push(format!("  {token}: fake only")),
            (false, true) => out.push(format!("  {token}: live only")),
        }
    }
    if out.is_empty() {
        out.push("  identical format".to_owned());
    }
    out
}

fn unique_sorted(fake: &[String], live: &[String]) -> Vec<String> {
    let mut all: Vec<String> = fake.to_vec();
    all.extend(live.to_vec());
    all.sort();
    all.dedup();
    all
}

fn count_of(list: &[String], token: &str) -> usize {
    list.iter().filter(|t| t.as_str() == token).count()
}

async fn probe_plain(client: &Client, url: &str, case: &ProbeCase) -> Result<Vec<String>, String> {
    if case.stream {
        let lines = sse_post_client(client, url, &case.body).await?;
        return stream_shape(&lines);
    }
    let (code, text) = request(client, Method::POST, url, Some(&case.body)).await?;
    if code != 200 {
        return Err(format!("fake probe {} status {code}", case.name));
    }
    json_shape(&text)
}

async fn probe_authorized(
    client: &Client,
    url: &str,
    case: &ProbeCase,
    profile: &LiveProfile,
) -> Result<Vec<String>, String> {
    let builder = authorized_builder(client, url, &case.body, profile);
    if case.stream {
        let collected = sse_send(builder).await?;
        return stream_shape(&collected);
    }
    let text = authorized_text(builder, url).await?;
    json_shape(&text)
}

fn authorized_builder(
    client: &Client,
    url: &str,
    body: &str,
    profile: &LiveProfile,
) -> reqwest::RequestBuilder {
    client
        .post(url)
        .bearer_auth(profile.key.expose())
        .body(body.to_owned())
}

async fn authorized_text(builder: reqwest::RequestBuilder, url: &str) -> Result<String, String> {
    let response = builder
        .send()
        .await
        .map_err(|e| format!("POST {url} failed: {e:?}"))?;
    let code = response.status().as_u16();
    let text = response
        .text()
        .await
        .map_err(|e| format!("read response failed: {e}"))?;
    if code != 200 {
        return Err(format!("live model returned status {code}: {text}"));
    }
    Ok(text)
}
