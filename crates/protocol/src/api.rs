use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

pub const PROBLEM_JSON_CONTENT_TYPE: &str = "application/problem+json";
pub const SSE_CONTENT_TYPE: &str = "text/event-stream";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct ProblemJson {
    #[serde(rename = "type")]
    pub problem_type: String,
    pub title: String,
    pub status: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instance: Option<String>,
}

impl ProblemJson {
    #[must_use]
    pub fn new(status: u16, title: String, problem_type: String) -> Self {
        ProblemJson {
            problem_type,
            title,
            status,
            detail: None,
            instance: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(transparent)]
pub struct IdempotencyKey(String);

impl IdempotencyKey {
    pub fn parse(text: &str) -> Result<Self, String> {
        if text.is_empty() || text.len() > 200 {
            return Err("Idempotency-Key 長度要在 1 到 200 之間".to_owned());
        }
        if !text.bytes().all(|b| (0x20..=0x7E).contains(&b)) {
            return Err("Idempotency-Key 只能含可印的 ASCII".to_owned());
        }
        Ok(IdempotencyKey(text.to_owned()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(transparent)]
pub struct PageCursor(String);

impl PageCursor {
    pub fn parse(text: &str) -> Result<Self, String> {
        if text.is_empty() || text.len() > 200 {
            return Err("游標長度要在 1 到 200 之間".to_owned());
        }
        if !text
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        {
            return Err("游標只能含字母、數字、-、_".to_owned());
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

    #[test]
    fn test_api_problem_json_roundtrip_and_skips() {
        let p = ProblemJson::new(
            404,
            "找不到".to_owned(),
            "https://anybot.test/problems/not-found".to_owned(),
        );
        let json = serde_json::to_string(&p).unwrap();
        assert!(json.contains("\"type\""));
        assert!(!json.contains("detail"));
        let back: ProblemJson = serde_json::from_str(&json).unwrap();
        assert_eq!(back, p);
    }

    #[test]
    fn test_api_problem_json_detail_roundtrip() {
        let mut p = ProblemJson::new(
            400,
            "格式錯".to_owned(),
            "https://anybot.test/problems/bad".to_owned(),
        );
        p.detail = Some("詳細".to_owned());
        let json = serde_json::to_string(&p).unwrap();
        let back: ProblemJson = serde_json::from_str(&json).unwrap();
        assert_eq!(back.detail.as_deref(), Some("詳細"));
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
