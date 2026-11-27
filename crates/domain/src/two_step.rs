use ring::hmac;

pub const STEP_MS: i64 = 30_000;
pub const CODE_DIGITS: usize = 6;
pub const WINDOW_STEPS: i64 = 1;
pub const STEP_REUSE_MS: i64 = 120_000;
pub const CHALLENGE_TTL_MS: i64 = 300_000;
pub const CHALLENGE_MAX_TRIES: i32 = 3;
pub const SECRET_BYTES: usize = 20;
pub const RECOVERY_CODE_COUNT: usize = 8;
pub const RECOVERY_CODE_BYTES: usize = 5;
pub const RECOVERY_CODE_LEN: usize = 10;
pub const ISSUER: &str = "AnyBot";

const CODE_MODULUS: u32 = 1_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StepOutOfRange;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodeInput {
    Totp(String),
    Recovery(String),
    Malformed,
}

#[must_use]
pub fn classify_code(raw: &str) -> CodeInput {
    let text = raw.trim();
    if text.len() == CODE_DIGITS && text.bytes().all(|b| b.is_ascii_digit()) {
        return CodeInput::Totp(text.to_owned());
    }
    if text.len() == RECOVERY_CODE_LEN && text.bytes().all(|b| b.is_ascii_hexdigit()) {
        return CodeInput::Recovery(text.to_ascii_lowercase());
    }
    CodeInput::Malformed
}

pub fn time_step(now_ms: i64) -> Result<i64, StepOutOfRange> {
    now_ms.checked_div(STEP_MS).ok_or(StepOutOfRange)
}

pub fn totp_code(secret: &[u8], step: i64) -> Result<String, StepOutOfRange> {
    let counter = u64::try_from(step).map_err(|_| StepOutOfRange)?;
    let key = hmac::Key::new(hmac::HMAC_SHA1_FOR_LEGACY_USE_ONLY, secret);
    let tag = hmac::sign(&key, &counter.to_be_bytes());
    let bytes = tag.as_ref();
    let offset = usize::from(bytes.last().ok_or(StepOutOfRange)? & 0x0f);
    let end = offset.checked_add(4).ok_or(StepOutOfRange)?;
    let slice = bytes.get(offset..end).ok_or(StepOutOfRange)?;
    let window = <[u8; 4]>::try_from(slice).map_err(|_| StepOutOfRange)?;
    let value = u32::from_be_bytes(window) & 0x7fff_ffff;
    let code = value.checked_rem(CODE_MODULUS).ok_or(StepOutOfRange)?;
    Ok(format!("{code:06}"))
}

pub fn matching_step(
    secret: &[u8],
    now_ms: i64,
    code: &str,
) -> Result<Option<i64>, StepOutOfRange> {
    let now = time_step(now_ms)?;
    let mut found = None;
    for offset in [-WINDOW_STEPS, 0, WINDOW_STEPS] {
        let step = now.checked_add(offset).ok_or(StepOutOfRange)?;
        if step < 0 {
            continue;
        }
        let candidate = totp_code(secret, step)?;
        if same_text(&candidate, code) && found.is_none() {
            found = Some(step);
        }
    }
    Ok(found)
}

#[must_use]
pub fn same_text(left: &str, right: &str) -> bool {
    left.len() == right.len()
        && left
            .bytes()
            .zip(right.bytes())
            .fold(0u8, |acc, (a, b)| acc | (a ^ b))
            == 0
}

#[must_use]
pub fn reuse_floor(now_ms: i64) -> i64 {
    now_ms.saturating_sub(STEP_REUSE_MS)
}

#[must_use]
pub fn challenge_open(created_at_ms: i64, tries: i32, now_ms: i64) -> bool {
    now_ms.saturating_sub(created_at_ms) < CHALLENGE_TTL_MS && tries < CHALLENGE_MAX_TRIES
}

#[must_use]
pub fn recovery_code(bytes: &[u8; RECOVERY_CODE_BYTES]) -> String {
    data_encoding::HEXLOWER.encode(bytes)
}

#[must_use]
pub fn secret_base32(secret: &[u8]) -> String {
    data_encoding::BASE32_NOPAD.encode(secret)
}

#[must_use]
pub fn otpauth_uri(account: &str, secret: &[u8]) -> String {
    let label: String = url::form_urlencoded::byte_serialize(account.as_bytes()).collect();
    format!(
        "otpauth://totp/{ISSUER}:{label}?secret={}&issuer={ISSUER}&period=30&digits=6",
        secret_base32(secret)
    )
}

#[cfg(test)]
#[path = "two_step_tests.rs"]
mod tests;
