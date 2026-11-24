use thiserror::Error;

#[derive(Error, Debug)]
pub enum FoundationError {
    #[error("設定缺少必填項目：{0}")]
    ConfigMissing(&'static str),
}

#[derive(Clone)]
pub struct Secret(String);

impl Secret {
    #[must_use]
    pub fn from_untrusted(raw: String) -> Self {
        Secret(raw)
    }

    #[must_use]
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for Secret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Secret(遮罩)")
    }
}

#[must_use]
pub fn now_epoch_seconds() -> i64 {
    time::OffsetDateTime::now_utc().unix_timestamp()
}

pub fn read_bootstrap_file(path: &str) -> Result<String, std::io::Error> {
    std::fs::read_to_string(path)
}

#[cfg(test)]
mod tests {
    use super::Secret;

    #[test]
    fn secret_is_redacted_in_debug() {
        let s = Secret::from_untrusted("top-secret".to_owned());
        assert_eq!(format!("{s:?}"), "Secret(遮罩)");
        assert_eq!(s.expose(), "top-secret");
    }
}
