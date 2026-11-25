use serde::{Deserialize, Serialize};

pub const PROBLEM_JSON_CONTENT_TYPE: &str = "application/problem+json";
pub const PROBLEM_TYPE_BASE: &str = "https://anybot.test/problems";
pub const SSE_CONTENT_TYPE: &str = "text/event-stream";
pub const PROBLEM_CODE_BAD_REQUEST: &str = "problem-bad-request";
pub const PROBLEM_CODE_INTERNAL: &str = "problem-internal";
pub const PROBLEM_TITLE_BAD_REQUEST: &str = "The request was malformed.";
pub const PROBLEM_TITLE_INTERNAL: &str = "The server hit an internal error.";

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
#[serde(transparent)]
pub struct MessageParams(std::collections::BTreeMap<String, serde_json::Value>);

impl MessageParams {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    #[must_use]
    pub fn get(&self, key: &str) -> Option<&serde_json::Value> {
        self.0.get(key)
    }

    #[must_use]
    pub fn from_pairs(items: impl IntoIterator<Item = (String, serde_json::Value)>) -> Self {
        MessageParams(items.into_iter().collect())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
pub struct ProblemJson {
    #[serde(rename = "type")]
    #[cfg_attr(
        feature = "schema",
        schema(example = "https://anybot.test/problems/not-found")
    )]
    pub problem_type: String,
    #[cfg_attr(feature = "schema", schema(example = "The request was malformed."))]
    pub title: String,
    #[cfg_attr(feature = "schema", schema(example = 400))]
    pub status: u16,
    #[serde(rename = "code")]
    #[cfg_attr(feature = "schema", schema(example = "problem-bad-request"))]
    pub code: String,
    #[cfg_attr(feature = "schema", schema(value_type = MessageParams))]
    pub params: MessageParams,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "schema", schema(example = "developer detail text"))]
    pub detail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "schema", schema(example = "/v1/health"))]
    pub instance: Option<String>,
}

impl ProblemJson {
    #[must_use]
    pub fn new(status: u16, title: String, problem_type: String, code: String) -> Self {
        ProblemJson {
            problem_type,
            title,
            status,
            code,
            params: MessageParams::default(),
            detail: None,
            instance: None,
        }
    }

    #[must_use]
    pub fn with_params(mut self, params: MessageParams) -> Self {
        self.params = params;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
#[serde(transparent)]
pub struct IdempotencyKey(String);

impl IdempotencyKey {
    pub fn parse(text: &str) -> Result<Self, String> {
        if text.is_empty() || text.len() > 200 {
            return Err("Idempotency-Key length must be between 1 and 200".to_owned());
        }
        if !text.bytes().all(|b| (0x20..=0x7E).contains(&b)) {
            return Err("Idempotency-Key must contain only printable ASCII".to_owned());
        }
        Ok(IdempotencyKey(text.to_owned()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
#[serde(transparent)]
pub struct PageCursor(String);

impl PageCursor {
    pub fn parse(text: &str) -> Result<Self, String> {
        if text.is_empty() || text.len() > 200 {
            return Err("cursor length must be between 1 and 200".to_owned());
        }
        if !text
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        {
            return Err("cursor must contain only letters, digits, - and _".to_owned());
        }
        Ok(PageCursor(text.to_owned()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn problem(status: u16, code: &str) -> ProblemJson {
        ProblemJson::new(
            status,
            PROBLEM_TITLE_BAD_REQUEST.to_owned(),
            format!("{PROBLEM_TYPE_BASE}/bad-request"),
            code.to_owned(),
        )
    }

    #[test]
    fn test_api_problem_json_roundtrip_and_skips() {
        let p = ProblemJson::new(
            404,
            PROBLEM_TITLE_BAD_REQUEST.to_owned(),
            format!("{PROBLEM_TYPE_BASE}/not-found"),
            PROBLEM_CODE_BAD_REQUEST.to_owned(),
        );
        let json = serde_json::to_string(&p).unwrap();
        assert!(json.contains("\"type\""));
        assert!(json.contains("\"code\""));
        assert!(!json.contains("detail"));
        assert!(json.contains("\"params\":{}"), "{json}");
        let back: ProblemJson = serde_json::from_str(&json).unwrap();
        assert_eq!(back, p);
    }

    #[test]
    fn test_api_problem_json_detail_roundtrip() {
        let mut p = problem(400, PROBLEM_CODE_BAD_REQUEST);
        p.detail = Some("detail text".to_owned());
        let json = serde_json::to_string(&p).unwrap();
        let back: ProblemJson = serde_json::from_str(&json).unwrap();
        assert_eq!(back.detail.as_deref(), Some("detail text"));
    }

    #[test]
    fn test_api_problem_json_params_roundtrip() {
        let p = problem(400, PROBLEM_CODE_BAD_REQUEST).with_params(MessageParams::from_pairs([(
            "header".to_owned(),
            serde_json::Value::String("Idempotency-Key".to_owned()),
        )]));
        let json = serde_json::to_string(&p).unwrap();
        assert!(json.contains("\"params\""));
        let back: ProblemJson = serde_json::from_str(&json).unwrap();
        assert_eq!(
            back.params.get("header"),
            Some(&serde_json::Value::String("Idempotency-Key".to_owned()))
        );
        assert_eq!(back, p);
    }

    #[test]
    fn test_api_idempotency_key_bounds_and_chars() {
        assert!(IdempotencyKey::parse("abc-123").is_ok());
        assert!(IdempotencyKey::parse("").is_err());
        assert!(IdempotencyKey::parse(&"x".repeat(201)).is_err());
        assert!(IdempotencyKey::parse("a\tb").is_err());
        let k = IdempotencyKey::parse("k1").unwrap();
        assert_eq!(serde_json::to_string(&k).unwrap(), "\"k1\"");
    }

    #[test]
    fn test_api_page_cursor_bounds_and_chars() {
        assert!(PageCursor::parse("cursor-1_2").is_ok());
        assert!(PageCursor::parse("").is_err());
        assert!(PageCursor::parse("a b").is_err());
        assert!(PageCursor::parse(&"c".repeat(201)).is_err());
    }
}
