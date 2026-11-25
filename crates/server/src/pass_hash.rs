use std::sync::Arc;

use argon2::password_hash::{rand_core::OsRng, PasswordHasher, SaltString, Verify, Encode};
use argon2::{Argon2, Algorithm, Version};
use sha2::{Digest, Sha256};
use tokio::sync::Semaphore;

use anybot_foundation::Secret;

const ARGON2_M: u32 = 19 * 1024;
const ARGON2_T: u32 = 2;
const ARGON2_PARALLEL: u32 = 1;
pub const HASH_CONCURRENCY: usize = 4;
const DUMMY_PASSWORD: &str = "0123456789";

pub struct PassHash {
    argon2: Argon2,
    permits: Arc<Semaphore>,
    dummy_hash: String,
}

impl PassHash {
    pub fn new() -> Result<Self, String> {
        let argon2 = Argon2::new(Algorithm::Argon2id, Version::Argon2v1)
            .with_m_cost(ARGON2_M)
            .with_time_cost(ARGON2_T)
            .with_parallelism(ARGON2_PARALLEL);
        let dummy_hash = Self::hash_with(&argon2, DUMMY_PASSWORD)?;
        Ok(PassHash {
            argon2,
            permits: Arc::new(Semaphore::new(HASH_CONCURRENCY)),
            dummy_hash,
        })
    }

    pub async fn hash(&self, password: &Secret) -> Result<String, String> {
        let _permit = self
            .permits
            .acquire()
            .await
            .map_err(|e| e.to_string())?;
        Self::hash_with(&self.argon2, password.expose())
    }

    pub async fn verify(&self, password: &Secret, stored: &str) -> Result<bool, String> {
        let _permit = self
            .permits
            .acquire()
            .await
            .map_err(|e| e.to_string())?;
        Self::verify_with(&self.argon2, password.expose(), stored)
    }

    pub async fn verify_dummy(&self, password: &Secret) -> Result<bool, String> {
        let _permit = self
            .permits
            .acquire()
            .await
            .map_err(|e| e.to_string())?;
        Self::verify_with(&self.argon2, password.expose(), &self.dummy_hash)
    }

    fn hash_with(argon2: &Argon2, password: &str) -> Result<String, String> {
        let salt = SaltString::generate(&mut OsRng);
        argon2
            .hash_password(password.as_bytes(), &salt)
            .map(|hash| hash.to_string())
            .map_err(|e| e.to_string())
    }

    fn verify_with(argon2: &Argon2, password: &str, stored: &str) -> Result<bool, String> {
        let parsed = argon2::password_hash::PasswordHash::new(stored)
            .map_err(|e| e.to_string())?;
        match argon2.verify(password.as_bytes(), &parsed) {
            Ok(()) => Ok(true),
            Err(argon2::password_hash::PasswordVerifyError::Mismatch) => Ok(false),
            Err(e) => Err(e.to_string()),
        }
    }
}

pub fn key_digest(key: &Secret) -> String {
    let mut hasher = Sha256::new();
    hasher.update(key.expose().as_bytes());
    let out = hasher.finalize();
    out.iter().map(|byte| format!("{byte:02x}")).collect()
}

impl Default for PassHash {
    fn default() -> Self {
        PassHash::new().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn secret(text: &str) -> Secret {
        Secret::from_untrusted(text.to_owned())
    }

    #[test]
    fn hash_round_trip_is_argon2id() {
        let hasher = PassHash::new().unwrap();
        let stored = hasher.hash(&secret("hunter22")).unwrap();
        assert!(stored.starts_with("$argon2id$"), "algorithm leak");
        assert!(hasher.verify(&secret("hunter22"), &stored).unwrap());
        assert!(!hasher.verify(&secret("hunter23"), &stored).unwrap());
    }

    #[test]
    fn digest_is_hex_sha256() {
        assert_eq!(key_digest(&secret("k")).len(), 64);
        assert_eq!(key_digest(&secret("k")), key_digest(&secret("k")));
        assert_ne!(key_digest(&secret("k")), key_digest(&secret("l")));
    }
}
