use ring::{aead, hkdf, hmac};

use anybot_foundation::MasterKey;

pub const KEY_ID_LEN: usize = 8;
pub const CHECK_ALGORITHM: &str = "hkdf-sha256/hmac-sha256/v1";

const ROOT_SALT: &[u8] = b"anybot.master-key.v1";
const INFO_CHECK: &[u8] = b"anybot.check.v1";
const INFO_KEY_ID: &[u8] = b"anybot.key-id.v1";
const CHECK_MESSAGE: &[u8] = b"anybot master key check";
const KEY_ID_MESSAGE: &[u8] = b"anybot key id";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SecretError {
    Derive,
    Random,
    Unreadable(String),
    WrongKey,
    Rejected,
}

impl std::fmt::Display for SecretError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SecretError::Derive => f.write_str("secret key derivation failed"),
            SecretError::Random => f.write_str("operating system randomness unavailable"),
            SecretError::Unreadable(reason) => write!(f, "sealed secret unreadable: {reason}"),
            SecretError::WrongKey => f.write_str("sealed secret belongs to another master key"),
            SecretError::Rejected => f.write_str(
                "sealed secret does not open here: moved to another table, row or column, or altered",
            ),
        }
    }
}

pub struct SecretKeys {
    root: hkdf::Prk,
    key_id: [u8; KEY_ID_LEN],
    check: hmac::Key,
}

impl std::fmt::Debug for SecretKeys {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SecretKeys(masked)")
    }
}

impl SecretKeys {
    pub fn from_master(master: &MasterKey) -> Result<Self, SecretError> {
        let root = hkdf::Salt::new(hkdf::HKDF_SHA256, ROOT_SALT).extract(master.expose_bytes());
        let check = expand_hmac(&root, INFO_CHECK)?;
        let id_key = expand_hmac(&root, INFO_KEY_ID)?;
        let tag = hmac::sign(&id_key, KEY_ID_MESSAGE);
        let head = tag.as_ref().get(..KEY_ID_LEN).ok_or(SecretError::Derive)?;
        let mut key_id = [0u8; KEY_ID_LEN];
        key_id.copy_from_slice(head);
        Ok(SecretKeys {
            root,
            key_id,
            check,
        })
    }

    #[must_use]
    pub fn key_id(&self) -> [u8; KEY_ID_LEN] {
        self.key_id
    }

    #[must_use]
    pub fn key_id_hex(&self) -> String {
        data_encoding::HEXLOWER.encode(&self.key_id)
    }

    #[must_use]
    pub fn check_value(&self) -> Vec<u8> {
        hmac::sign(&self.check, CHECK_MESSAGE).as_ref().to_vec()
    }

    #[must_use]
    pub fn check_matches(&self, stored: &[u8]) -> bool {
        hmac::verify(&self.check, CHECK_MESSAGE, stored).is_ok()
    }

    pub(crate) fn hmac_key(&self, info: &[u8]) -> Result<hmac::Key, SecretError> {
        expand_hmac(&self.root, info)
    }

    pub(crate) fn aead_key(&self, info: &[u8]) -> Result<aead::LessSafeKey, SecretError> {
        let infos = [info];
        let okm = self
            .root
            .expand(&infos, &aead::AES_256_GCM)
            .map_err(|_| SecretError::Derive)?;
        Ok(aead::LessSafeKey::new(aead::UnboundKey::from(okm)))
    }
}

fn expand_hmac(root: &hkdf::Prk, info: &[u8]) -> Result<hmac::Key, SecretError> {
    let infos = [info];
    let okm = root
        .expand(&infos, hmac::HMAC_SHA256)
        .map_err(|_| SecretError::Derive)?;
    Ok(hmac::Key::from(okm))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keys() -> SecretKeys {
        SecretKeys::from_master(&MasterKey::generate().unwrap()).unwrap()
    }

    #[test]
    fn check_value_matches_only_its_own_master_key() {
        let a = keys();
        let b = keys();
        assert!(a.check_matches(&a.check_value()));
        assert!(!a.check_matches(&b.check_value()));
        assert_ne!(a.key_id(), b.key_id());
    }

    #[test]
    fn check_value_and_key_id_do_not_contain_the_master_key() {
        let master = MasterKey::generate().unwrap();
        let derived = SecretKeys::from_master(&master).unwrap();
        let raw = master.expose_bytes();
        let check = derived.check_value();
        assert_eq!(check.len(), 32);
        assert_ne!(check.as_slice(), raw.as_slice());
        assert!(!raw.windows(KEY_ID_LEN).any(|w| w == derived.key_id()));
    }
}
