use zeroize::Zeroize;

pub struct SecretBytes(Vec<u8>);

impl SecretBytes {
    #[must_use]
    pub fn from_untrusted(raw: Vec<u8>) -> Self {
        SecretBytes(raw)
    }

    #[must_use]
    pub fn expose(&self) -> &[u8] {
        &self.0
    }
}

impl std::fmt::Debug for SecretBytes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SecretBytes(masked)")
    }
}

impl Drop for SecretBytes {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secret_bytes_debug_is_masked() {
        let s = SecretBytes::from_untrusted(vec![1, 2, 3]);
        assert_eq!(format!("{s:?}"), "SecretBytes(masked)");
        assert_eq!(s.expose(), &[1, 2, 3]);
    }
}
