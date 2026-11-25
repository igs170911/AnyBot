use anybot_foundation as _;
use anybot_models::{RunPlace, classify_url, first_token_wait};
use bytes as _;
use futures_util as _;
use reqwest as _;
use serde_json as _;
use test_harness as _;
use tokio as _;
use url as _;

#[test]
fn t_models_where_1() {
    let samples = [
        ("http://localhost:11434", RunPlace::Local),
        ("http://192.168.1.20:8080", RunPlace::Lan),
        ("http://nas.local", RunPlace::Lan),
        ("http://gpu", RunPlace::Lan),
        ("https://box.tail1234.ts.net", RunPlace::Lan),
        ("https://api.x.ai", RunPlace::Cloud),
        ("http://172.32.0.1", RunPlace::Cloud),
    ];
    let places: Vec<RunPlace> = samples.iter().map(|(url, _)| classify_url(url)).collect();
    let expected: Vec<RunPlace> = samples.iter().map(|(_, place)| *place).collect();
    assert_eq!(places, expected);
}

#[test]
fn t_chat_stall_1() {
    let waits =
        [20_000, 30_000, 31_000, 100_000, 300_000].map(|tokens| first_token_wait(tokens).as_secs());
    assert_eq!(waits, [120, 120, 122, 260, 600]);
}
