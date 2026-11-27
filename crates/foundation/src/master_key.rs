use data_encoding::{HEXLOWER, HEXLOWER_PERMISSIVE};
use zeroize::Zeroize;

use crate::error::{CODE_BOOTSTRAP_MASTER_KEY_MALFORMED, FoundationError, MessageRef};
use crate::secret::Secret;

pub const MASTER_KEY_BYTES: usize = 32;
pub const MASTER_KEY_HEX_LEN: usize = 64;
pub const MASTER_KEY_NAME: &str = "master_key";

#[derive(Clone)]
pub struct MasterKey([u8; MASTER_KEY_BYTES]);

impl MasterKey {
    pub fn generate() -> Result<Self, FoundationError> {
        let mut bytes = [0u8; MASTER_KEY_BYTES];
        getrandom::fill(&mut bytes)
            .map_err(|e| FoundationError::RandomUnavailable(e.to_string()))?;
        Ok(MasterKey(bytes))
    }

    pub fn from_hex(text: &str) -> Result<Self, FoundationError> {
        if text.len() != MASTER_KEY_HEX_LEN || !text.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(malformed());
        }
        let mut decoded = HEXLOWER_PERMISSIVE
            .decode(text.as_bytes())
            .map_err(|_| malformed())?;
        let mut bytes = [0u8; MASTER_KEY_BYTES];
        if decoded.len() != MASTER_KEY_BYTES {
            decoded.zeroize();
            return Err(malformed());
        }
        bytes.copy_from_slice(&decoded);
        decoded.zeroize();
        Ok(MasterKey(bytes))
    }

    #[must_use]
    pub fn expose_bytes(&self) -> &[u8; MASTER_KEY_BYTES] {
        &self.0
    }

    #[must_use]
    pub fn bootstrap_line(&self) -> Secret {
        Secret::from_untrusted(format!("{MASTER_KEY_NAME}={}", HEXLOWER.encode(&self.0)))
    }
}

fn malformed() -> FoundationError {
    FoundationError::BootstrapMalformed(
        MessageRef::new(CODE_BOOTSTRAP_MASTER_KEY_MALFORMED).with_text("name", MASTER_KEY_NAME),
    )
}

impl std::fmt::Debug for MasterKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("MasterKey(masked)")
    }
}

impl Drop for MasterKey {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_keys_differ_and_print_one_bootstrap_line() {
        let a = MasterKey::generate().unwrap();
        let b = MasterKey::generate().unwrap();
        assert_ne!(a.expose_bytes(), b.expose_bytes());
        let line = a.bootstrap_line();
        let hex = line.expose().strip_prefix("master_key=").unwrap();
        assert_eq!(hex.len(), MASTER_KEY_HEX_LEN);
        assert!(hex.bytes().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(format!("{a:?}"), "MasterKey(masked)");
    }

    #[test]
    fn hex_round_trip_accepts_both_cases_and_rejects_bad_shapes() {
        let upper = "AB".repeat(32);
        let key = MasterKey::from_hex(&upper).unwrap();
        assert_eq!(key.expose_bytes(), &[0xAB; 32]);
        assert!(MasterKey::from_hex(&"a".repeat(63)).is_err());
        assert!(MasterKey::from_hex(&"a".repeat(65)).is_err());
        assert!(MasterKey::from_hex(&format!("{}g", "a".repeat(63))).is_err());
    }
}
