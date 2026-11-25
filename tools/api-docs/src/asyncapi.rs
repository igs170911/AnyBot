use schemars::schema_for;
use serde_json::Value;

use anybot_protocol::{KnownEventKind, PROTOCOL_VERSION, RecordedEvent};

const CHANNEL_ADDRESS: &str = "/v1/tasks/{taskId}/events/stream";
const DEF_MARKER: &str = "#/$defs/";

fn event_kind_variants() -> Result<Vec<Value>, String> {
    let schema = serde_json::to_value(schema_for!(KnownEventKind)).map_err(|e| e.to_string())?;
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

fn move_defs(node: &mut Value, defs: &mut serde_json::Map<String, Value>) {
    if let Some(obj) = node.as_object_mut() {
        if let Some(local) = obj.remove("$defs")
            && let Value::Object(map) = local
        {
            defs.extend(map);
        }
        for (_, child) in obj.iter_mut() {
            move_defs(child, defs);
        }
    }
    if let Some(list) = node.as_array_mut() {
        for child in list.iter_mut() {
            move_defs(child, defs);
        }
    }
}

fn rewrite_refs(node: &mut Value) {
    if let Some(obj) = node.as_object_mut() {
        if let Some(Value::String(ref_text)) = obj.get("$ref")
            && let Some(rest) = ref_text.clone().strip_prefix(DEF_MARKER)
        {
            obj.insert(
                "$ref".to_owned(),
                Value::String(format!("#/components/schemas/{rest}")),
            );
        }
        for (_, child) in obj.iter_mut() {
            rewrite_refs(child);
        }
    }
    if let Some(list) = node.as_array_mut() {
        for child in list.iter_mut() {
            rewrite_refs(child);
        }
    }
}

fn scalar_example(type_name: &str) -> Value {
    match type_name {
        "string" => Value::String("2026-01-01T00:00:00Z".to_owned()),
        "integer" => Value::from(1),
        "number" => Value::from(1.5),
        "boolean" => Value::from(true),
        _ => Value::Null,
    }
}

fn type_names(property: &Value) -> Vec<String> {
    match property.get("type") {
        Some(Value::String(single)) => vec![single.clone()],
        Some(Value::Array(list)) => list
            .iter()
            .filter_map(|v| v.as_str().map(ToOwned::to_owned))
            .collect(),
        _ => vec![],
    }
}

fn add_scalar_examples(node: &mut Value) {
    if let Some(obj) = node.as_object_mut() {
        if let Some(properties) = obj.get_mut("properties")
            && let Some(map) = properties.as_object_mut()
        {
            for (_, property) in map.iter_mut() {
                let types = type_names(property);
                let scalar = types.iter().any(|t| scalar_example(t) != Value::Null);
                let missing = property.get("examples").is_none();
                if scalar && !types.contains(&"null".to_owned()) && missing {
                    let first = types
                        .iter()
                        .find(|t| scalar_example(t) != Value::Null)
                        .map_or(Value::Null, |t| scalar_example(t));
                    if let Some(prop_obj) = property.as_object_mut() {
                        prop_obj.insert("examples".to_owned(), Value::Array(vec![first]));
                    }
                }
            }
        }
        for (_, child) in obj.iter_mut() {
            add_scalar_examples(child);
        }
    }
    if let Some(list) = node.as_array_mut() {
        for child in list.iter_mut() {
            add_scalar_examples(child);
        }
    }
}

fn normalize_schema(schema: &mut Value, defs: &mut serde_json::Map<String, Value>) {
    move_defs(schema, defs);
    rewrite_refs(schema);
    for def in defs.values_mut() {
        rewrite_refs(def);
    }
    add_scalar_examples(schema);
    if schema.get("type").is_none()
        && schema.get("$ref").is_none()
        && let Some(obj) = schema.as_object_mut()
    {
        obj.insert("type".to_owned(), Value::String("object".to_owned()));
    }
}

fn component_messages(defs: &mut serde_json::Map<String, Value>) -> Result<Value, String> {
    let mut map = serde_json::Map::new();
    for variant in event_kind_variants()? {
        let name = variant_name(&variant)?;
        let mut payload = variant.clone();
        normalize_schema(&mut payload, defs);
        map.insert(
            name.clone(),
            serde_json::json!({
                "name": name,
                "title": name,
                "summary": "執行事件協定的事件種類",
                "payload": payload,
            }),
        );
    }
    Ok(Value::Object(map))
}

fn base_info() -> Value {
    serde_json::json!({
        "title": "AnyBot 執行事件協定",
        "version": PROTOCOL_VERSION.to_string(),
        "description": "伺服器推給執行端與畫面的執行事件，走 SSE；內容 schema 由 Rust 型別產生。",
        "contact": { "name": "AnyBot 開發團隊", "url": "http://127.0.0.1:8787/v1/health" },
        "license": {
            "name": "MIT OR Apache-2.0",
            "url": "https://opensource.org/license/mit/",
        },
    })
}

pub fn document() -> Result<String, String> {
    let mut run_schema =
        serde_json::to_value(schema_for!(RecordedEvent)).map_err(|e| e.to_string())?;
    let mut defs = serde_json::Map::new();
    normalize_schema(&mut run_schema, &mut defs);
    let component_messages = component_messages(&mut defs)?;
    let message_entries: serde_json::Map<String, Value> = component_messages
        .as_object()
        .map(|m| m.keys().map(|k| (k.clone(), ref_entry(k))).collect())
        .unwrap_or_default();
    let message_refs: Vec<Value> = message_entries.values().cloned().collect();
    let mut schemas = serde_json::Map::new();
    schemas.insert("runEventEnvelope".to_owned(), run_schema);
    for (name, def) in defs {
        schemas.insert(name, def);
    }
    let doc = serde_json::json!({
        "asyncapi": "3.0.0",
        "info": base_info(),
        "components": {
            "schemas": Value::Object(schemas),
            "messages": component_messages,
            "securitySchemes": {
                "bearer": {
                    "type": "httpApiKey",
                    "name": "Authorization",
                    "in": "header",
                },
            },
        },
        "channels": {
            "runEvents": {
                "address": CHANNEL_ADDRESS,
                "description": "依任務推送事件；每則有遞增編號。",
                "messages": Value::Object(message_entries.clone()),
            }
        },
        "operations": {
            "streamRunEvents": {
                "action": "receive",
                "title": "接收事件串流",
                "description": "客戶端從伺服器接收事件串流",
                "operationId": "stream_run_events",
                "tags": [{ "name": "events", "description": "執行事件協定的事件種類" }],
                "channel": { "$ref": "#/channels/runEvents" },
                "security": [{
                    "type": "httpApiKey",
                    "name": "Authorization",
                    "in": "header",
                    "description": "M3 佔位；Bearer 金鑰。",
                }],
                "messages": message_refs,
            }
        },
    });
    yaml_serde::to_string(&doc).map_err(|e| e.to_string())
}

fn ref_entry(name: &String) -> Value {
    Value::Object(serde_json::Map::from_iter([(
        "$ref".to_owned(),
        Value::String(format!("#/components/messages/{name}")),
    )]))
}
