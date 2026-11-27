use ring::hmac;

const STEP_SECONDS_MS: u64 = 30_000;
const SHOWN_DIGITS: u32 = 6;
const LAST_NIBBLE: u8 = 0x0f;
const SIGN_MASK: u32 = 0x7fff_ffff;

#[derive(Clone)]
pub struct FakeAuthenticator {
    secret: Vec<u8>,
}

impl FakeAuthenticator {
    #[must_use]
    pub fn from_bytes(secret: &[u8]) -> Self {
        FakeAuthenticator {
            secret: secret.to_vec(),
        }
    }

    pub fn from_base32(text: &str) -> Result<Self, String> {
        let secret = data_encoding::BASE32_NOPAD
            .decode(text.as_bytes())
            .map_err(|e| format!("authenticator secret is not base32: {e}"))?;
        Ok(FakeAuthenticator { secret })
    }

    #[must_use]
    pub fn secret(&self) -> &[u8] {
        &self.secret
    }

    pub fn code_at(&self, unix_ms: u64) -> Result<String, String> {
        self.code_for_counter(counter_of(unix_ms)?, SHOWN_DIGITS)
    }

    pub fn previous_code_at(&self, unix_ms: u64) -> Result<String, String> {
        let counter = counter_of(unix_ms)?
            .checked_sub(1)
            .ok_or("no step before the epoch".to_owned())?;
        self.code_for_counter(counter, SHOWN_DIGITS)
    }

    pub fn next_code_at(&self, unix_ms: u64) -> Result<String, String> {
        let counter = counter_of(unix_ms)?
            .checked_add(1)
            .ok_or("step counter overflow".to_owned())?;
        self.code_for_counter(counter, SHOWN_DIGITS)
    }

    pub fn code_for_counter(&self, counter: u64, digits: u32) -> Result<String, String> {
        let key = hmac::Key::new(hmac::HMAC_SHA1_FOR_LEGACY_USE_ONLY, &self.secret);
        let mac = hmac::sign(&key, &counter.to_be_bytes());
        let binary = dynamic_truncation(mac.as_ref())?;
        let modulus = 10u32
            .checked_pow(digits)
            .ok_or("too many digits".to_owned())?;
        let value = binary
            .checked_rem(modulus)
            .ok_or("zero modulus".to_owned())?;
        let width = usize::try_from(digits).map_err(|e| e.to_string())?;
        Ok(format!("{value:0width$}"))
    }
}

fn counter_of(unix_ms: u64) -> Result<u64, String> {
    unix_ms
        .checked_div(STEP_SECONDS_MS)
        .ok_or("zero step".to_owned())
}

fn dynamic_truncation(mac: &[u8]) -> Result<u32, String> {
    let last = mac.last().ok_or("empty hmac".to_owned())?;
    let offset = usize::from(last & LAST_NIBBLE);
    let mut binary: u32 = 0;
    let mut taken = 0u8;
    for byte in mac.iter().skip(offset).take(4) {
        binary = binary.checked_shl(8).ok_or("shift overflow".to_owned())? | u32::from(*byte);
        taken = taken.saturating_add(1);
    }
    if taken != 4 {
        return Err(format!("hmac too short at offset {offset}"));
    }
    Ok(binary & SIGN_MASK)
}

#[cfg(test)]
mod tests {
    use super::*;

    const RFC_SECRET: &[u8] = b"12345678901234567890";

    #[test]
    fn fake_authenticator_matches_rfc6238_appendix_b_sha1_vectors() {
        let vectors: [(u64, &str); 6] = [
            (59, "94287082"),
            (1_111_111_109, "07081804"),
            (1_111_111_111, "14050471"),
            (1_234_567_890, "89005924"),
            (2_000_000_000, "69279037"),
            (20_000_000_000, "65353130"),
        ];
        let authenticator = FakeAuthenticator::from_bytes(RFC_SECRET);
        for (seconds, want) in vectors {
            let counter = seconds.checked_div(30).unwrap();
            let got = authenticator.code_for_counter(counter, 8).unwrap();
            assert_eq!(got, want, "RFC 6238 vector at {seconds}s");
        }
    }

    #[test]
    fn fake_authenticator_previous_step_differs_from_current() {
        let authenticator = FakeAuthenticator::from_bytes(RFC_SECRET);
        let now = 1_700_000_000_000;
        let current = authenticator.code_at(now).unwrap();
        let previous = authenticator.previous_code_at(now).unwrap();
        assert_eq!(current.len(), 6);
        assert_ne!(current, previous);
        let earlier = now.checked_sub(30_000).unwrap();
        assert_eq!(authenticator.code_at(earlier).unwrap(), previous);
    }
}
