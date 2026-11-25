#[must_use]
pub fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| byte_hex(*byte)).collect()
}

#[must_use]
fn byte_hex(byte: u8) -> String {
    format!("{byte:02x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_encodes_each_byte_lowercase() {
        assert_eq!(to_hex(&[0x00, 0xff, 0x0a]), "00ff0a");
    }
}
