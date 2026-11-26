use sha2::{Digest, Sha256};

#[must_use]
pub fn content_digest(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    let sum = hasher.finalize();
    hex_prefix(&sum, 8)
}

const HEX_DIGITS: [char; 16] = [
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f',
];

fn hex_prefix(sum: &[u8], count: usize) -> String {
    let mut out = String::new();
    for byte in sum.iter().take(count) {
        let high = usize::from(*byte >> 4);
        let low = usize::from(*byte & 15);
        let Some(high_ch) = HEX_DIGITS.get(high) else {
            continue;
        };
        let Some(low_ch) = HEX_DIGITS.get(low) else {
            continue;
        };
        out.push(*high_ch);
        out.push(*low_ch);
    }
    out
}

#[cfg(test)]
#[path = "digest_tests.rs"]
mod tests;
