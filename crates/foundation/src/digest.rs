use std::fmt;

use sha2::{Digest, Sha256};

use crate::error::FoundationError;

const HEX_LEN: usize = 64;

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
)]
#[serde(transparent)]
pub struct ContentDigest(String);

impl ContentDigest {
    #[must_use]
    pub fn sha256(bytes: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        Self(hex_lower(&hasher.finalize()))
    }

    #[must_use]
    pub fn hex(&self) -> &str {
        &self.0
    }

    pub fn parse(text: &str) -> Result<Self, FoundationError> {
        if text.len() != HEX_LEN || !is_lower_hex(text) {
            return Err(FoundationError::IdParse(
                "content digest must be 64 lowercase hex characters".to_owned(),
            ));
        }
        Ok(Self(text.to_owned()))
    }
}

fn is_lower_hex(text: &str) -> bool {
    text.chars()
        .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c))
}

fn hex_lower(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().saturating_mul(2));
    for b in bytes {
        push_nibble(&mut out, b >> 4);
        push_nibble(&mut out, b & 0x0F);
    }
    out
}

fn push_nibble(out: &mut String, value: u8) {
    let Some(c) = HEX_NIBBLES.get(usize::from(value)) else {
        return;
    };
    out.push(*c);
}

const HEX_NIBBLES: [char; 16] = [
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f',
];

impl fmt::Display for ContentDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_digest_sha256_of_empty_input_is_known_value() {
        let got = ContentDigest::sha256(b"");
        assert_eq!(
            got.hex(),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn test_content_digest_parse_round_trips_and_blocks_bad_shapes() {
        let value = ContentDigest::sha256(b"x");
        let parsed = ContentDigest::parse(value.hex()).unwrap();
        assert_eq!(parsed, value);
        assert!(ContentDigest::parse("nope").is_err());
        assert!(ContentDigest::parse(&"A".repeat(64)).is_err());
        assert!(ContentDigest::parse(&"a".repeat(63)).is_err());
    }
}
