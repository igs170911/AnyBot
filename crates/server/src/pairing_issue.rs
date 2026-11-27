use axum::http::StatusCode;

use anybot_domain::pairing_rules::{CODE_TTL_MS, PENDING_CAP, code_from_bytes};
use anybot_foundation::Secret;
use anybot_messages::MessageCode;
use anybot_protocol::PairingCodeReply;
use anybot_store_server::pairing_codes::{InsertOutcome, NewCode, insert_code};
use anybot_store_server::{HashPurpose, Pool, SecretKeys};

use crate::computer_view::signed;
use crate::req_write::{CallerDenied, denied, internal_detail, internal_sqlx, internal_text};

const DRAW_ATTEMPTS: usize = 16;

pub(crate) fn draw_code() -> Result<String, CallerDenied> {
    for _attempt in 0..DRAW_ATTEMPTS {
        let mut bytes = [0u8; 4];
        getrandom::fill(&mut bytes).map_err(internal_text)?;
        if let Some(code) = code_from_bytes(bytes) {
            return Ok(code);
        }
    }
    Err(internal_detail("no unbiased pairing code after many draws"))
}

pub(crate) async fn issue_code(
    keys: &SecretKeys,
    pool: &Pool,
    account: &str,
    now_ms: u64,
) -> Result<PairingCodeReply, CallerDenied> {
    let expires = now_ms.saturating_add(CODE_TTL_MS);
    for _attempt in 0..DRAW_ATTEMPTS {
        let code = draw_code()?;
        let hash = keys
            .keyed_hash(
                HashPurpose::PairingCode,
                "",
                &Secret::from_untrusted(code.clone()),
            )
            .map_err(internal_text)?;
        let id = uuid::Uuid::now_v7().to_string();
        let outcome = insert_code(
            pool,
            &NewCode {
                id: &id,
                account,
                algorithm: &hash.algorithm,
                digest: &hash.digest,
                created_at_ms: signed(now_ms)?,
                expires_at_ms: signed(expires)?,
            },
            PENDING_CAP,
        )
        .await
        .map_err(internal_sqlx)?;
        match outcome {
            InsertOutcome::Stored => {
                return Ok(PairingCodeReply {
                    code,
                    expires_at_ms: signed(expires)?,
                });
            }
            InsertOutcome::Full => {
                return Err(denied(
                    StatusCode::TOO_MANY_REQUESTS,
                    MessageCode::ComputersPairFull,
                ));
            }
            InsertOutcome::Duplicate => {}
        }
    }
    Err(internal_detail("pairing code space is crowded"))
}
