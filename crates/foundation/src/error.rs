use std::collections::BTreeMap;

use serde::Serialize;
use thiserror::Error;

pub type MessageParams = BTreeMap<String, serde_json::Value>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MessageRef {
    pub code: String,
    pub params: MessageParams,
}

impl MessageRef {
    #[must_use]
    pub fn new(code: &str) -> Self {
        MessageRef {
            code: code.to_owned(),
            params: MessageParams::new(),
        }
    }

    #[must_use]
    pub fn with_text(mut self, key: &str, value: &str) -> Self {
        self.params
            .insert(key.to_owned(), serde_json::Value::String(value.to_owned()));
        self
    }

    #[must_use]
    pub fn with_int(mut self, key: &str, value: i64) -> Self {
        self.params.insert(key.to_owned(), serde_json::json!(value));
        self
    }
}

pub const CODE_BOOTSTRAP_NOT_FOUND: &str = "server-bootstrap-file-missing";
pub const CODE_BOOTSTRAP_PERMISSIONS_READ_FAILED: &str = "server-bootstrap-permissions-read-failed";
pub const CODE_BOOTSTRAP_READ_FAILED: &str = "server-bootstrap-read-failed";
pub const CODE_BOOTSTRAP_PERMISSIONS_TOO_OPEN: &str = "server-bootstrap-permissions-too-open";
pub const CODE_BOOTSTRAP_URL_MALFORMED: &str = "server-bootstrap-url-malformed";
pub const CODE_BOOTSTRAP_URL_SCHEME: &str = "server-bootstrap-url-scheme";
pub const CODE_BOOTSTRAP_CREDENTIALS_MISSING: &str = "server-bootstrap-credentials-missing";
pub const CODE_BOOTSTRAP_HOST_MISSING: &str = "server-bootstrap-host-missing";
pub const CODE_BOOTSTRAP_DATABASE_MISSING: &str = "server-bootstrap-database-missing";
pub const CODE_BOOTSTRAP_MISSING: &str = "server-bootstrap-missing";
pub const CODE_CLIENT_DATA_DIR_MISSING: &str = "client-data-dir-missing";
pub const CODE_SERVER_STARTUP_LISTENING: &str = "server-startup-listening";

#[derive(Error, Debug, PartialEq)]
pub enum FoundationError {
    #[error("configuration missing: {0}")]
    ConfigMissing(&'static str),

    #[error("bootstrap not found ({0:?})")]
    BootstrapNotFound(MessageRef),

    #[error("bootstrap permissions ({0:?})")]
    BootstrapPermissions(MessageRef),

    #[error("bootstrap malformed ({0:?})")]
    BootstrapMalformed(MessageRef),

    #[error("id parse failed: {0}")]
    IdParse(String),

    #[error("data dir missing ({0:?})")]
    DataDirMissing(MessageRef),
}

impl FoundationError {
    #[must_use]
    pub fn message_ref(&self) -> Option<MessageRef> {
        match self {
            Self::BootstrapNotFound(r)
            | Self::BootstrapPermissions(r)
            | Self::BootstrapMalformed(r)
            | Self::DataDirMissing(r) => Some(r.clone()),
            Self::ConfigMissing(_) | Self::IdParse(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_ref_serializes_code_and_params() {
        let r = MessageRef::new(CODE_BOOTSTRAP_NOT_FOUND)
            .with_text("path", "/tmp/x")
            .with_int("port", 8787);
        let json = serde_json::to_value(&r).unwrap();
        assert_eq!(
            json.get("code"),
            Some(&serde_json::json!("server-bootstrap-file-missing"))
        );
        let params = json.get("params").and_then(serde_json::Value::as_object);
        assert_eq!(
            params.and_then(|m| m.get("path")),
            Some(&serde_json::json!("/tmp/x"))
        );
        assert_eq!(
            params.and_then(|m| m.get("port")),
            Some(&serde_json::json!(8787))
        );
    }

    #[test]
    fn test_foundation_error_message_ref_carries_only_localized_variants() {
        let e = FoundationError::BootstrapPermissions(
            MessageRef::new(CODE_BOOTSTRAP_PERMISSIONS_TOO_OPEN)
                .with_text("path", "p")
                .with_text("mode", "644"),
        );
        let r = e.message_ref().unwrap();
        assert_eq!(r.code, CODE_BOOTSTRAP_PERMISSIONS_TOO_OPEN);
        assert_eq!(
            r.params.get("mode").map(serde_json::Value::as_str),
            Some(Some("644"))
        );
        assert!(
            FoundationError::IdParse("x".to_owned())
                .message_ref()
                .is_none()
        );
    }

    #[test]
    fn test_foundation_error_display_is_developer_english() {
        let e = FoundationError::BootstrapNotFound(
            MessageRef::new(CODE_BOOTSTRAP_NOT_FOUND).with_text("path", "start.toml"),
        );
        let text = format!("{e}");
        assert!(text.contains("bootstrap not found"));
        assert!(text.contains("server-bootstrap-file-missing"));
    }
}
