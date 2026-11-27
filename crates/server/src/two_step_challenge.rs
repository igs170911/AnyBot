use axum::http::StatusCode;
use axum::response::Response;
use sha2::{Digest, Sha256};

use anybot_protocol::TwoStepChallenge;
use anybot_store_server::two_step_challenges::{NewChallenge, insert_challenge};

use crate::api_login_issue::SessionIssue;
use crate::caller::CallerDenied;
use crate::req_write::{Outcome, internal_sqlx, record_outcome, value_of};
use crate::two_step_check::now_ms;

pub(crate) const CREDENTIAL_COOKIE: &str = "cookie";
pub(crate) const CREDENTIAL_BEARER: &str = "bearer";

#[must_use]
pub(crate) fn challenge_hash(token: &str) -> Vec<u8> {
    Sha256::digest(token.as_bytes()).to_vec()
}

#[must_use]
pub(crate) fn challenge_fingerprint(id_hash: &[u8]) -> Vec<u8> {
    let value = serde_json::json!({ "challenge": crate::hex::to_hex(id_hash) });
    value.to_string().into_bytes()
}

pub(crate) async fn open_challenge(issue: &SessionIssue<'_>) -> Result<Response, CallerDenied> {
    let token = crate::session_key::random_key()?;
    let id_hash = challenge_hash(&token);
    let credential = if issue.cookie_mode {
        CREDENTIAL_COOKIE
    } else {
        CREDENTIAL_BEARER
    };
    insert_challenge(
        issue.pool,
        &NewChallenge {
            id_hash: &id_hash,
            account: issue.account,
            credential,
            label: issue.label,
            at_ms: now_ms(issue.state)?,
        },
    )
    .await
    .map_err(internal_sqlx)?;
    let value = value_of(&TwoStepChallenge {
        challenge: token,
        account: issue.account.to_owned(),
    })?;
    record_outcome(
        issue.pool,
        issue.state,
        &Outcome {
            key: issue.key,
            raw: issue.raw,
            scope: issue.scope,
            status: StatusCode::ACCEPTED,
            value: &value,
            marker: true,
        },
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fingerprint_names_only_the_challenge_hash() {
        let hash = challenge_hash("token");
        let text = String::from_utf8(challenge_fingerprint(&hash)).unwrap();
        assert!(text.starts_with("{\"challenge\":\""));
        assert!(!text.contains("token"));
        assert_ne!(challenge_hash("token"), challenge_hash("other"));
    }
}
