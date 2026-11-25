use std::path::{Path, PathBuf};

use reqwest::Client;
use test_harness::{
    FakeModel, ModelFormat, Step, model_start_with_format, plain_client, request, sse_post_client,
    sse_send,
};

use crate::probe::{chat_url, live_client, normal_request, stream_request, tool_request};
use crate::profile::{LiveProfile, REPORT_REL, load_profile, profile_dir};
use crate::shape::{json_shape, stream_shape};

pub const SCENARIOS: [&str; 3] = ["normal", "stream", "tool-call"];

pub struct Shapes {
    pub fake: Vec<Vec<String>>,
    pub live: Vec<Vec<String>>,
}

pub async fn fake_shapes(
    model: &FakeModel,
    port: u16,
    name: &str,
) -> Result<Vec<Vec<String>>, String> {
    model.script_set(vec![
        Step::ReplyText {
            text: "probe".to_owned(),
            stream: false,
        },
        Step::ReplyText {
            text: "probe".to_owned(),
            stream: true,
        },
        Step::CallTool {
            name: crate::probe::TOOL_NAME.to_owned(),
            args: serde_json::json!({}),
            stream: false,
        },
    ]);
    let url = format!("http://127.0.0.1:{port}/v1/chat/completions");
    let client = plain_client()?;
    let normal = probe_json(&client, &url, &normal_request(name)).await?;
    let stream = probe_stream(&client, &url, &stream_request(name)).await?;
    let tool = probe_json(&client, &url, &tool_request(name)).await?;
    Ok(vec![normal, stream, tool])
}

async fn probe_json(client: &Client, url: &str, body: &str) -> Result<Vec<String>, String> {
    let (code, text) = request(client, reqwest::Method::POST, url, Some(body)).await?;
    if code != 200 {
        return Err(format!("POST {url} returned status {code}"));
    }
    json_shape(&text)
}

async fn probe_stream(client: &Client, url: &str, body: &str) -> Result<Vec<String>, String> {
    let lines = sse_post_client(client, url, body).await?;
    stream_shape(&lines)
}

pub async fn live_shapes(
    profile: &LiveProfile,
    client: &Client,
    name: &str,
) -> Result<Vec<Vec<String>>, String> {
    let url = chat_url(&profile.base_url);
    let normal = probe_json_authorized(client, &url, &normal_request(name), profile).await?;
    let stream = probe_stream_authorized(client, &url, &stream_request(name), profile).await?;
    let tool = probe_json_authorized(client, &url, &tool_request(name), profile).await?;
    Ok(vec![normal, stream, tool])
}

async fn probe_json_authorized(
    client: &Client,
    url: &str,
    body: &str,
    profile: &LiveProfile,
) -> Result<Vec<String>, String> {
    let builder = client
        .post(url)
        .bearer_auth(profile.key.expose())
        .body(body.to_owned());
    let response = builder
        .send()
        .await
        .map_err(|e| format!("POST {url} failed: {e:?}"))?;
    let code = response.status().as_u16();
    if code != 200 {
        let detail = response
            .text()
            .await
            .map_err(|e| format!("read failure body failed: {e}"))?;
        return Err(format!("live model returned status {code}: {detail}"));
    }
    let text = response
        .text()
        .await
        .map_err(|e| format!("read response failed: {e}"))?;
    json_shape(&text)
}

async fn probe_stream_authorized(
    client: &Client,
    url: &str,
    body: &str,
    profile: &LiveProfile,
) -> Result<Vec<String>, String> {
    let builder = client
        .post(url)
        .bearer_auth(profile.key.expose())
        .body(body.to_owned());
    let collected = sse_send(builder).await?;
    stream_shape(&collected)
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
    for (index, name) in SCENARIOS.iter().copied().enumerate() {
        let fake = shapes.fake.get(index).cloned().unwrap_or_default();
        let live = shapes.live.get(index).cloned().unwrap_or_default();
        out.push(format!("scenario {name}:"));
        out.extend(diff_tokens(&fake, &live));
    }
    out
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

pub async fn run() -> Result<(), String> {
    let repo = test_harness::repo_root();
    let profile = load_profile(&repo)?;
    let client = live_client(&profile)?;
    let (model, port) = model_start_with_format(ModelFormat::Vllm).await?;
    let fake = fake_shapes(&model, port, &profile.model).await?;
    let live = live_shapes(&profile, &client, &profile.model).await?;
    let shapes = Shapes { fake, live };
    let path = write_report(&repo, &shapes)?;
    let mut stdout = std::io::stdout();
    let mut tail = String::new();
    for line in report_lines(&shapes) {
        tail.push_str(&line);
        tail.push('\n');
    }
    tail.push_str(
        format!(
            "report written to {}
",
            path.display()
        )
        .as_str(),
    );
    std::io::Write::write_fmt(&mut stdout, format_args!("{tail}")).map_err(|e| e.to_string())?;
    Ok(())
}
