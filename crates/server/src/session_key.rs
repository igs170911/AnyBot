use getrandom::fill;

use crate::hex::to_hex;

pub const KEY_BYTES: usize = 32;

pub fn random_key() -> Result<String, crate::req_write::CallerDenied> {
    let mut bytes = [0u8; KEY_BYTES];
    fill(&mut bytes).map_err(|e| crate::req_write::internal_detail(&e.to_string()))?;
    Ok(to_hex(&bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_are_unique_hex() {
        let a = random_key().unwrap();
        let b = random_key().unwrap();
        assert_eq!(a.len(), KEY_BYTES * 2);
        assert_ne!(a, b);
    }
}
