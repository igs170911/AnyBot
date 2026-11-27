use anybot_app_server as _;
use anybot_domain as _;
use anybot_foundation as _;
use anybot_messages as _;
use anybot_server as _;
use anybot_store_server as _;
use axum as _;
use clap as _;
use rpassword as _;
use serde_json as _;
use sqlx as _;
use test_harness as _;
use tokio as _;
use tracing as _;

use anybot_app_test::serve::serve_start;
use anybot_app_test::{dump_database, found_digest, found_spelling};
use anybot_server_test::computers_support::{RedeemAttempt, code_text, computer_login, redeem};
use anybot_server_test::make_admin;

fn code_as_field(dump: &str, code: &str) -> bool {
    [
        format!("\t{code}\t"),
        format!("\t{code}\n"),
        format!("'{code}'"),
    ]
    .iter()
    .any(|spelled| dump.contains(spelled.as_str()))
}

#[tokio::test]
async fn pairing_codes_and_computer_keys_are_not_stored_readable() -> Result<(), String> {
    let served = serve_start("pairing_secrets_dump").await?;
    let phone = make_admin(&served).await?;
    let code = code_text(&served, &phone, "k-code").await?;
    let attempt = RedeemAttempt {
        code: &code,
        name: "L",
        key: "r-1",
        address: None,
    };
    let paired = redeem(&served, &attempt).await?;
    assert_eq!(paired.status, 200, "{}", paired.json);
    let paired_key = paired.bearer()?;
    let unredeemed = code_text(&served, &phone, "k-code-2").await?;
    let login = computer_login(&served, "desk", "k-desk").await?;
    let dump = dump_database(&served.db)?;
    for code in [&code, &unredeemed] {
        assert!(
            !code_as_field(&dump, code),
            "pairing code {code} in the dump"
        );
        assert_eq!(
            found_digest(&dump, code.as_bytes()),
            None,
            "unkeyed digest of {code}"
        );
    }
    for key in [&paired_key, &login.bearer] {
        assert_eq!(
            found_spelling(&dump, key.as_bytes()),
            None,
            "computer key in the dump"
        );
    }
    let lengths: Vec<i32> = sqlx::query_scalar("SELECT length(digest) FROM pairing_codes")
        .fetch_all(&served.pool)
        .await
        .map_err(|e| e.to_string())?;
    assert!(
        !lengths.is_empty() && lengths.iter().all(|length| *length == 32),
        "{lengths:?}"
    );
    served.stop().await?;
    Ok(())
}
