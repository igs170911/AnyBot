use anybot_foundation::ContentDigest;

pub const RESULT_ID_PREFIX: &str = "r";
pub const RESULT_ID_HEX_CHARS: usize = 16;

#[must_use]
pub fn result_id(digest: &ContentDigest) -> String {
    let prefix: String = digest.hex().chars().take(RESULT_ID_HEX_CHARS).collect();
    format!("{RESULT_ID_PREFIX}{prefix}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn result_id_starts_with_r_and_sixteen_hex_chars() {
        let digest = ContentDigest::sha256(b"hello world");
        let id = result_id(&digest);
        let prefix: String = digest.hex().chars().take(16).collect();
        assert_eq!(id, format!("r{prefix}"));
        assert_eq!(id.len(), 17);
    }
}
