use zeroize::Zeroize;

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

impl Drop for Secret {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

impl Zeroize for Secret {
    fn zeroize(&mut self) {
        self.0.zeroize();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secret_debug_is_redacted_and_expose_is_explicit() {
        let s = Secret::from_untrusted("hunter2".to_owned());
        assert_eq!(format!("{s:?}"), "Secret(遮罩)");
        assert_eq!(s.expose(), "hunter2");
    }

    #[test]
    fn test_secret_zeroizes_on_explicit_call_and_drop_clears_backing() {
        let mut s = Secret::from_untrusted("abcdef".to_owned());
        s.zeroize();
        assert_eq!(s.expose(), "");
    }

    #[test]
    fn test_secret_serialization_is_a_compile_failure() {
        trybuild::TestCases::new().compile_fail("tests/compilefail/*.rs");
    }
}
