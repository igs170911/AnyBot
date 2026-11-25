use std::collections::BTreeSet;

use crate::{asyncapi, openapi};

fn kind_names() -> BTreeSet<String> {
    let schema = schemars::schema_for!(anybot_protocol::KnownEventKind);
    let value = serde_json::to_value(schema).unwrap();
    let one_of = value.get("oneOf").unwrap().as_array().unwrap();
    let mut out = BTreeSet::new();
    for variant in one_of {
        let kind = variant
            .get("properties")
            .unwrap()
            .get("kind")
            .unwrap()
            .get("const")
            .unwrap()
            .as_str()
            .unwrap();
        out.insert(kind.to_owned());
    }
    out
}

fn message_names(doc: &serde_json::Value) -> BTreeSet<String> {
    doc.get("components")
        .and_then(|c| c.get("messages"))
        .and_then(|m| m.as_object())
        .map(|m| m.keys().cloned().collect())
        .unwrap_or_default()
}

#[test]
fn test_asyncapi_covers_every_event_kind() {
    let text = asyncapi::document().unwrap();
    let doc: serde_json::Value = yaml_serde::from_str(&text).unwrap();
    assert_eq!(message_names(&doc), kind_names());
    assert_eq!(kind_names().len(), 10);
}

#[test]
fn test_openapi_documents_health_path_with_required_metadata() {
    let text = openapi::document().unwrap();
    let doc: serde_json::Value = yaml_serde::from_str(&text).unwrap();
    let op = doc
        .get("paths")
        .and_then(|p| p.get("/v1/health"))
        .and_then(|h| h.get("get"))
        .unwrap();
    assert_eq!(
        op.get("operationId").and_then(|v| v.as_str()),
        Some("get_health")
    );
    assert!(op.get("summary").is_some());
    assert!(op.get("description").is_some());
    assert!(op.get("tags").is_some());
    let responses = op.get("responses").unwrap().as_object().unwrap();
    assert!(responses.contains_key("200"));
    assert!(responses.contains_key("400"));
    assert!(responses.contains_key("500"));
    for (code, response) in responses {
        assert!(
            response.get("content").is_some(),
            "response {code} lacks content"
        );
    }
}

#[test]
fn test_openapi_is_openapi_31() {
    let text = openapi::document().unwrap();
    let doc: serde_json::Value = yaml_serde::from_str(&text).unwrap();
    assert_eq!(doc.get("openapi").and_then(|v| v.as_str()), Some("3.1.0"));
}

fn operations(doc: &serde_json::Value) -> Vec<&serde_json::Value> {
    let mut out = Vec::new();
    let Some(paths) = doc.get("paths").and_then(|v| v.as_object()) else {
        return out;
    };
    for item in paths.values() {
        let Some(methods) = item.as_object() else {
            continue;
        };
        for (method, op) in methods {
            if matches!(method.as_str(), "get" | "post" | "put" | "patch" | "delete") {
                out.push(op);
            }
        }
    }
    out
}

#[test]
fn test_openapi_error_responses_only_use_problem_json() {
    let text = openapi::document().unwrap();
    let doc: serde_json::Value = yaml_serde::from_str(&text).unwrap();
    for op in operations(&doc) {
        let Some(responses) = op.get("responses").and_then(|v| v.as_object()) else {
            continue;
        };
        for (code, response) in responses {
            let Some(status) = code.parse::<u16>().ok() else {
                continue;
            };
            if status < 400 {
                continue;
            }
            let content = response.get("content").and_then(|v| v.as_object()).unwrap();
            let keys: Vec<&String> = content.keys().collect();
            assert_eq!(
                keys,
                vec!["application/problem+json"],
                "{code} must use only application/problem+json"
            );
        }
    }
}

#[test]
fn test_openapi_every_response_has_example() {
    let text = openapi::document().unwrap();
    let doc: serde_json::Value = yaml_serde::from_str(&text).unwrap();
    for op in operations(&doc) {
        let Some(responses) = op.get("responses").and_then(|v| v.as_object()) else {
            continue;
        };
        for (code, response) in responses {
            assert!(
                response
                    .get("content")
                    .and_then(|c| c.as_object())
                    .and_then(|m| m.values().next())
                    .and_then(|media| media.get("example"))
                    .is_some(),
                "response {code} lacks an example"
            );
        }
    }
}
