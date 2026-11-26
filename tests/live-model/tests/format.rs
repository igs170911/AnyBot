use std::path::PathBuf;

use anybot_foundation as _;
use anybot_models::{ProviderKind, resolve_endpoint};
use live_model::profile::{PROFILE_REL, load_profile};
use live_model::shape::{json_shape, stream_shape};
use live_model::side::{Shapes, diff_tokens_public, report_lines};
use reqwest as _;
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
    let model_dir = repo.join(".local").join("live-model");
    std::fs::create_dir_all(&model_dir).unwrap();
    std::fs::remove_dir_all(&model_dir).unwrap();
    let result = load_profile(&repo);
    let Err(message) = result else {
        panic!("missing profile must fail");
    };
    assert!(message.contains(PROFILE_REL), "{message}");
    for field in ["base_url", "model", "ca_file", "key_file", "always_thinks"] {
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
            vec!["same".to_owned()],
        ],
    };
    let lines = report_lines(&shapes);
    let text = lines.join("\n");
    assert!(text.contains("b: fake only"), "{text}");
    assert!(text.contains("c: live only"), "{text}");
    assert_eq!(text.matches("identical format").count(), 1, "{text}");
    assert!(text.contains("missing fake"), "{text}");
    assert!(
        !text.contains("key"),
        "report must not mention keys: {text}"
    );
}

#[test]
fn live_product_lines_list_a_missing_side() {
    let lines = live_model::product::product_lines(&[], &[]);
    let text = lines.join("\n");
    assert!(text.contains("product client:"), "{text}");
    assert!(text.contains("missing fake and live"), "{text}");
    assert!(!text.contains("identical"), "{text}");
}

#[test]
fn live_report_file_includes_product_client() {
    let repo = scratch_repo("live-report-product");
    let shapes = Shapes {
        fake: vec![vec!["a".to_owned()]],
        live: vec![vec!["a".to_owned()]],
    };
    let product = vec![
        "product client:".to_owned(),
        "  normal: missing live".to_owned(),
    ];
    let path = live_model::side::write_report(&repo, &shapes, &product).unwrap();
    let text = std::fs::read_to_string(path).unwrap();
    assert!(text.contains("product client:"), "{text}");
    assert!(text.contains("missing live"), "{text}");
}

#[tokio::test]
async fn live_run_uses_https_fake_model() {
    let started = test_harness::model_start_https("model.invalid")
        .await
        .unwrap();
    started.model.script_set(live_model::cases::product_steps());
    let trusted = anybot_foundation::TrustedCerts::parse_pem(&started.cert.pem).unwrap();
    let outcome =
        live_model::product::https_product_shapes(started.port, "local", "model.invalid", &trusted)
            .await;
    assert!(outcome.failed.is_none(), "{:?}", outcome.failed);
    assert_eq!(outcome.side.len(), 4);
}

#[tokio::test]
async fn live_side_label_names_the_failing_side() {
    let endpoint = resolve_endpoint(
        ProviderKind::Custom,
        "https://127.0.0.1:9/v1",
        "k",
        "local",
        None,
    );
    let outcome = live_model::product::complete_labeled(endpoint, "live").await;
    let message = outcome.failed.unwrap_or_default();
    assert!(message.contains("live product"), "{message}");
    assert!(!message.contains("fake product"), "{message}");
}

#[test]
fn live_diff_missing_side_is_not_identical() {
    let missing = diff_tokens_public(&[], &["x".to_owned()]);
    assert_eq!(missing, vec!["  missing fake".to_owned()]);
    let both = diff_tokens_public(&[], &[]);
    assert_eq!(both, vec!["  missing fake and live".to_owned()]);
    assert!(!both.iter().any(|line| line.contains("identical")));
}

#[test]
fn live_diff_tokens_public_wrapper_exists() {
    let out = diff_tokens_public(&["x".to_owned()], &["y".to_owned()]);
    assert_eq!(out.len(), 2);
    let same = diff_tokens_public(&["x".to_owned()], &["x".to_owned()]);
    assert_eq!(same, vec!["  identical format".to_owned()]);
}
