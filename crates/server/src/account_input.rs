use anybot_domain::account_name::{is_allowed_password, normalize_account_name};

pub fn normalize_account(raw: &str) -> Result<String, String> {
    normalize_account_name(raw)
}

#[must_use]
pub fn password_long_enough(raw: &str) -> bool {
    is_allowed_password(raw)
}
