use anybot_foundation as _;
use anybot_models::{RunPlace, classify_url, first_token_wait};
use bytes as _;
use futures_util as _;
use proptest as _;
use reqwest as _;
use rustls as _;
use serde as _;
use serde_json as _;
use test_harness as _;
use tokio as _;
use trybuild as _;
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
fn t_models_where_4() {
    let samples = [
        ("http://[::1]:8080", RunPlace::Local),
        ("http://[fd00::1]", RunPlace::Lan),
        ("http://[fe80::1]", RunPlace::Lan),
        ("http://[2001:db8::1]", RunPlace::Cloud),
        ("http://[::ffff:192.168.1.20]", RunPlace::Lan),
        ("http://[::ffff:203.0.113.5]", RunPlace::Cloud),
        ("http://127.x.example", RunPlace::Cloud),
        ("http://10.x.example", RunPlace::Cloud),
        ("http://localhost.x.example", RunPlace::Cloud),
        ("http://192.168.1.20.x.example", RunPlace::Cloud),
        ("http://gpu.", RunPlace::Cloud),
        ("not a url", RunPlace::Cloud),
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
