use ring::hmac;

use anybot_foundation::Secret;

use crate::secret_keys::{SecretError, SecretKeys};

pub const HASH_ALGORITHM: &str = "hmac-sha256-v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HashPurpose {
    RecoveryCode,
    PairingCode,
}

impl HashPurpose {
    fn info(self) -> &'static [u8] {
        match self {
            HashPurpose::RecoveryCode => b"anybot.hash.recovery-code.v1",
            HashPurpose::PairingCode => b"anybot.hash.pairing-code.v1",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyedHash {
    pub algorithm: String,
    pub digest: Vec<u8>,
}

impl SecretKeys {
    pub fn keyed_hash(
        &self,
        purpose: HashPurpose,
        binding: &str,
        value: &Secret,
    ) -> Result<KeyedHash, SecretError> {
        let key = self.hmac_key(purpose.info())?;
        let tag = hmac::sign(&key, &hash_input(binding, value)?);
        Ok(KeyedHash {
            algorithm: HASH_ALGORITHM.to_owned(),
            digest: tag.as_ref().to_vec(),
        })
    }

    pub fn keyed_hash_matches(
        &self,
        purpose: HashPurpose,
        binding: &str,
        value: &Secret,
        stored: &KeyedHash,
    ) -> Result<bool, SecretError> {
        if stored.algorithm != HASH_ALGORITHM {
            return Err(SecretError::Unreadable(format!(
                "unknown hash algorithm {}",
                stored.algorithm
            )));
        }
        let key = self.hmac_key(purpose.info())?;
        Ok(hmac::verify(&key, &hash_input(binding, value)?, &stored.digest).is_ok())
    }
}

fn hash_input(binding: &str, value: &Secret) -> Result<Vec<u8>, SecretError> {
    let len = u32::try_from(binding.len()).map_err(|_| SecretError::Derive)?;
    let mut out = len.to_be_bytes().to_vec();
    out.extend_from_slice(binding.as_bytes());
    out.extend_from_slice(value.expose().as_bytes());
    Ok(out)
}

#[cfg(test)]
mod tests {
    use anybot_foundation::MasterKey;

    use super::*;

    fn keys() -> SecretKeys {
        SecretKeys::from_master(&MasterKey::generate().unwrap()).unwrap()
    }

    fn code() -> Secret {
        Secret::from_untrusted("0123456789".to_owned())
    }

    #[test]
    fn same_code_bound_to_other_accounts_hashes_differently() {
        let keys = keys();
        let alice = keys
            .keyed_hash(HashPurpose::RecoveryCode, "alice", &code())
            .unwrap();
        let bob = keys
            .keyed_hash(HashPurpose::RecoveryCode, "bob", &code())
            .unwrap();
        assert_ne!(alice.digest, bob.digest);
        assert!(
            !keys
                .keyed_hash_matches(HashPurpose::RecoveryCode, "bob", &code(), &alice)
                .unwrap()
        );
    }

    #[test]
    fn purposes_and_master_keys_hash_differently() {
        let keys_a = keys();
        let recovery = keys_a
            .keyed_hash(HashPurpose::RecoveryCode, "", &code())
            .unwrap();
        let pairing = keys_a
            .keyed_hash(HashPurpose::PairingCode, "", &code())
            .unwrap();
        let other = keys()
            .keyed_hash(HashPurpose::RecoveryCode, "", &code())
            .unwrap();
        assert_ne!(recovery.digest, pairing.digest);
        assert_ne!(recovery.digest, other.digest);
    }

    #[test]
    fn stored_algorithm_reads_back_and_matches() {
        let keys = keys();
        let stored = keys
            .keyed_hash(HashPurpose::RecoveryCode, "alice", &code())
            .unwrap();
        assert_eq!(stored.algorithm, "hmac-sha256-v1");
        assert_eq!(stored.digest.len(), 32);
        assert!(
            keys.keyed_hash_matches(HashPurpose::RecoveryCode, "alice", &code(), &stored)
                .unwrap()
        );
        let wrong = Secret::from_untrusted("0123456788".to_owned());
        assert!(
            !keys
                .keyed_hash_matches(HashPurpose::RecoveryCode, "alice", &wrong, &stored)
                .unwrap()
        );
    }

    #[test]
    fn unknown_algorithm_is_an_error() {
        let keys = keys();
        let mut stored = keys
            .keyed_hash(HashPurpose::RecoveryCode, "alice", &code())
            .unwrap();
        stored.algorithm = "sha256".to_owned();
        assert!(
            keys.keyed_hash_matches(HashPurpose::RecoveryCode, "alice", &code(), &stored)
                .is_err()
        );
    }
}
