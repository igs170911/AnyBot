use std::path::PathBuf;

use anybot_foundation as _;
use live_model::compare::{Shapes, diff_tokens_public, report_lines};
use live_model::profile::{PROFILE_REL, load_profile};
use live_model::shape::{json_shape, stream_shape};
use reqwest as _;
use rustls as _;
use rustls_pki_types as _;
use serde_json as _;
use test_harness as _;
use tokio as _;
use toml as _;

fn scratch_repo(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/scratch")
        .join(name)
}

#[test]
fn live_profile_missing_file_names_every_field() {
    let repo = scratch_repo("live-missing-profile");
    let model_dir = repo.join("target").join("live-model");
    std::fs::create_dir_all(&model_dir).unwrap();
    std::fs::remove_dir_all(&model_dir).unwrap();
    let result = load_profile(&repo);
    let Err(message) = result else {
        panic!("missing profile must fail");
    };
    assert!(message.contains(PROFILE_REL), "{message}");
    for field in ["base_url", "model", "ca_file", "key_file"] {
        assert!(message.contains(field), "{message}");
    }
}

#[test]
fn live_shape_lists_fields_of_a_plain_completion() {
    let body = r#"{"id":"a","object":"chat.completion","created":1,"model":"m","choices":[{"index":0,"message":{"role":"assistant","content":"hi"},"finish_reason":"stop"}],"usage":{"prompt_tokens":1}}"#;
    let Ok(shape) = json_shape(body) else {
        panic!("plain completion must parse");
    };
    assert!(
        shape.contains(&"object=chat.completion".to_owned()),
        "{shape:?}"
    );
    assert!(
        shape.contains(&"finish_reason=stop".to_owned()),
        "{shape:?}"
    );
    assert!(shape.contains(&"role".to_owned()), "{shape:?}");
}

#[test]
fn live_shape_orders_stream_chunks_and_end_marker() {
    let lines = vec![
        "data: {\"id\":\"a\",\"object\":\"chat.completion.chunk\",\"choices\":[{\"index\":0,\"delta\":{\"role\":\"assistant\"},\"finish_reason\":null}]}".to_owned(),
        "data: {\"id\":\"a\",\"object\":\"chat.completion.chunk\",\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"stop\"}]}".to_owned(),
        "data: [DONE]".to_owned(),
    ];
    let Ok(shape) = stream_shape(&lines) else {
        panic!("stream chunks must parse");
    };
    assert_eq!(
        shape.first().map(String::as_str),
        Some("object=chat.completion.chunk")
    );
    assert!(shape.contains(&"end=[DONE]".to_owned()), "{shape:?}");
    assert!(
        shape.contains(&"finish_reason=stop".to_owned()),
        "{shape:?}"
    );
}

#[test]
fn live_shape_maps_tool_call_structure() {
    let body = r#"{"object":"chat.completion","choices":[{"index":0,"message":{"role":"assistant","content":null,"tool_calls":[{"id":"c","type":"function","function":{"name":"f","arguments":"{}"}}]},"finish_reason":"tool_calls"}]}"#;
    let Ok(shape) = json_shape(body) else {
        panic!("tool call completion must parse");
    };
    assert!(
        shape.contains(&"finish_reason=tool_calls".to_owned()),
        "{shape:?}"
    );
    assert!(
        shape.contains(&"tool_calls:count=1".to_owned()),
        "{shape:?}"
    );
    assert!(shape.contains(&"function.name".to_owned()), "{shape:?}");
}

#[test]
fn live_report_marks_differences_and_matching_scenarios() {
    let shapes = Shapes {
        fake: vec![
            vec!["a".to_owned(), "b".to_owned()],
            vec!["a".to_owned()],
            vec![],
        ],
        live: vec![
            vec!["a".to_owned(), "c".to_owned()],
            vec!["a".to_owned()],
            vec![],
        ],
    };
    let lines = report_lines(&shapes);
    let text = lines.join("\n");
    assert!(text.contains("b: fake only"), "{text}");
    assert!(text.contains("c: live only"), "{text}");
    assert_eq!(text.matches("identical format").count(), 2, "{text}");
    assert!(
        !text.contains("key"),
        "report must not mention keys: {text}"
    );
}

#[test]
fn live_diff_tokens_public_wrapper_exists() {
    let out = diff_tokens_public(&["x".to_owned()], &["y".to_owned()]);
    assert_eq!(out.len(), 2);
    let same = diff_tokens_public(&["x".to_owned()], &["x".to_owned()]);
    assert_eq!(same, vec!["  identical format".to_owned()]);
}
