use std::fmt::Write as _;
use std::path::Path;

use sha2::Digest as _;

pub const OPENAPI_REL: &str = "api/openapi.yaml";
pub const ASYNCAPI_REL: &str = "api/asyncapi.yaml";
pub const CHANGELOG_REL: &str = "api/breaking-changes.log";

#[must_use]
pub fn baseline(openapi_bytes: &[u8], asyncapi_bytes: &[u8]) -> String {
    let mut hasher = sha2::Sha256::new();
    hasher.update(openapi_bytes);
    hasher.update(asyncapi_bytes);
    let out = hasher.finalize();
    out.iter().fold(String::new(), |mut acc, byte| {
        write!(acc, "{byte:02x}").unwrap_or_default();
        acc
    })
}

#[must_use = "the hash format check decides whether a changelog line is valid"]
pub fn is_hex64(text: &str) -> bool {
    text.chars().count() == 64 && text.bytes().all(|b| b.is_ascii_hexdigit())
}

#[must_use = "the bytes feed the baseline hash"]
pub fn read_bytes(path: &Path) -> Result<Vec<u8>, String> {
    std::fs::read(path).map_err(|e| format!("cannot read {}: {e}", path.display()))
}
