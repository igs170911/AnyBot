use schemars::schema_for;
use serde_json::Value;

use anybot_protocol::{EventKind, PROTOCOL_VERSION, RunEvent};

const CHANNEL_ADDRESS: &str = "/v1/tasks/{taskId}/events/stream";

fn event_kind_variants() -> Result<Vec<Value>, String> {
    let schema = serde_json::to_value(schema_for!(EventKind)).map_err(|e| e.to_string())?;
    let one_of = schema
        .get("oneOf")
        .and_then(|v| v.as_array())
        .ok_or("EventKind 的 schema 沒有 oneOf".to_owned())?;
    Ok(one_of.clone())
}

fn variant_name(variant: &Value) -> Result<String, String> {
    let kind = variant
        .get("properties")
        .and_then(|p| p.get("kind"))
        .ok_or("事件變體缺少 kind".to_owned())?;
    if let Some(name) = kind.get("const").and_then(|v| v.as_str()) {
        return Ok(name.to_owned());
    }
    kind.get("enum")
        .and_then(|v| v.as_array())
        .and_then(|a| a.first())
        .and_then(|v| v.as_str())
        .map(ToOwned::to_owned)
        .ok_or("事件變體缺少 kind 名稱".to_owned())
}

fn messages() -> Result<Value, String> {
    let mut map = serde_json::Map::new();
    for variant in event_kind_variants()? {
        let name = variant_name(&variant)?;
        let message = serde_json::json!({
            "name": name,
            "title": name,
            "summary": "執行事件協定的事件種類",
            "payload": variant,
        });
        map.insert(name, message);
    }
    Ok(Value::Object(map))
}

pub fn document() -> Result<String, String> {
    let run_schema = serde_json::to_value(schema_for!(RunEvent)).map_err(|e| e.to_string())?;
    let messages = messages()?;
    let names: Vec<String> = messages
        .as_object()
        .map(|m| m.keys().cloned().collect())
        .unwrap_or_default();
    let doc = serde_json::json!({
        "asyncapi": "3.0.0",
        "info": {
            "title": "AnyBot 執行事件協定",
            "version": PROTOCOL_VERSION.to_string(),
            "description": "伺服器推給執行端與畫面的執行事件，走 SSE；內容 schema 由 Rust 型別產生。",
        },
        "components": {
            "schemas": { "runEventEnvelope": run_schema },
            "messages": messages,
            "securitySchemes": {
                "bearer": { "type": "http", "scheme": "bearer" },
            },
        },
        "channels": {
            "runEvents": {
                "address": CHANNEL_ADDRESS,
                "description": "依任務推送事件；每則有遞增編號。",
                "messages": { "runEvent": { "payload": { "$ref": "#/components/schemas/runEventEnvelope" } } },
            }
        },
        "operations": {
            "streamRunEvents": {
                "action": "receive",
                "description": "客戶端從伺服器接收事件串流",
                "channel": { "$ref": "#/channels/runEvents" },
                "security": [{ "bearer": [] }],
                "messages": names
                    .iter()
                    .map(|n| Value::String(format!("#/components/messages/{n}")))
                    .collect::<Vec<Value>>(),
            }
        },
    });
    noyalib::to_string(&doc).map_err(|e| e.to_string())
}
