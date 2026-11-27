use ring::aead::{Aad, NONCE_LEN, Nonce};
use ring::rand::{SecureRandom, SystemRandom};

use anybot_foundation::{Secret, SecretBytes};

use crate::secret_keys::{KEY_ID_LEN, SecretError, SecretKeys};

pub const SEAL_VERSION: u8 = 1;

const INFO_SEAL: &[u8] = b"anybot.seal.aes-256-gcm.v1";
const AAD_PREFIX: &[u8] = b"anybot.seal.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SecretPlace<'a> {
    pub table: &'a str,
    pub column: &'a str,
    pub row: &'a str,
}

impl SecretPlace<'_> {
    fn aad(&self) -> Result<Vec<u8>, SecretError> {
        let mut out = AAD_PREFIX.to_vec();
        for part in [self.table, self.column, self.row] {
            let len = u32::try_from(part.len()).map_err(|_| SecretError::Derive)?;
            out.extend_from_slice(&len.to_be_bytes());
            out.extend_from_slice(part.as_bytes());
        }
        Ok(out)
    }
}

impl SecretKeys {
    pub fn seal(
        &self,
        place: &SecretPlace<'_>,
        plain: &SecretBytes,
    ) -> Result<Vec<u8>, SecretError> {
        let key = self.aead_key(INFO_SEAL)?;
        let mut nonce = [0u8; NONCE_LEN];
        SystemRandom::new()
            .fill(&mut nonce)
            .map_err(|_| SecretError::Random)?;
        let mut body = plain.expose().to_vec();
        key.seal_in_place_append_tag(
            Nonce::assume_unique_for_key(nonce),
            Aad::from(place.aad()?),
            &mut body,
        )
        .map_err(|_| SecretError::Derive)?;
        let mut sealed = vec![SEAL_VERSION];
        sealed.extend_from_slice(&self.key_id());
        sealed.extend_from_slice(&nonce);
        sealed.extend_from_slice(&body);
        Ok(sealed)
    }

    pub fn open(&self, place: &SecretPlace<'_>, sealed: &[u8]) -> Result<SecretBytes, SecretError> {
        let (version, rest) = sealed.split_first().ok_or_else(|| unreadable("empty"))?;
        if *version != SEAL_VERSION {
            return Err(unreadable(&format!("unknown seal version {version}")));
        }
        let (key_id, rest) = rest
            .split_at_checked(KEY_ID_LEN)
            .ok_or_else(|| unreadable("truncated key id"))?;
        if key_id != self.key_id() {
            return Err(SecretError::WrongKey);
        }
        let (nonce, body) = rest
            .split_at_checked(NONCE_LEN)
            .ok_or_else(|| unreadable("truncated nonce"))?;
        let nonce = Nonce::try_assume_unique_for_key(nonce).map_err(|_| unreadable("nonce"))?;
        let key = self.aead_key(INFO_SEAL)?;
        let mut buffer = body.to_vec();
        let opened = key
            .open_in_place(nonce, Aad::from(place.aad()?), &mut buffer)
            .map_err(|_| SecretError::Rejected)?
            .len();
        buffer.truncate(opened);
        Ok(SecretBytes::from_untrusted(buffer))
    }

    pub fn seal_text(
        &self,
        place: &SecretPlace<'_>,
        plain: &Secret,
    ) -> Result<Vec<u8>, SecretError> {
        self.seal(
            place,
            &SecretBytes::from_untrusted(plain.expose().as_bytes().to_vec()),
        )
    }

    pub fn open_text(&self, place: &SecretPlace<'_>, sealed: &[u8]) -> Result<Secret, SecretError> {
        let opened = self.open(place, sealed)?;
        let text = std::str::from_utf8(opened.expose()).map_err(|_| unreadable("not text"))?;
        Ok(Secret::from_untrusted(text.to_owned()))
    }
}

fn unreadable(reason: &str) -> SecretError {
    SecretError::Unreadable(reason.to_owned())
}

#[cfg(test)]
#[path = "secret_seal_tests.rs"]
mod tests;
