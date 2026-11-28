use anybot_app_server as _;
use anybot_app_test as _;
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

use anybot_app_test::http::{RequestSpec, get};
use anybot_app_test::serve::serve_start;
use anybot_server_test::make_admin;

#[tokio::test]
async fn status_names_the_admin_only_once_one_exists() -> Result<(), String> {
    let served = serve_start("status_admin_account").await?;
    let base = served.base_url();
    let before = get(&base, "/v1/server/status", &RequestSpec::new("{}")).await?;
    assert_eq!(before.status, 200, "{}", before.json);
    assert_eq!(
        before.json.get("admin_account"),
        Some(&serde_json::Value::Null),
        "{}",
        before.json
    );
    make_admin(&served).await?;
    let after = get(&base, "/v1/server/status", &RequestSpec::new("{}")).await?;
    assert_eq!(after.status, 200, "{}", after.json);
    assert_eq!(
        after.json.get("admin_account"),
        Some(&serde_json::json!("alice")),
        "{}",
        after.json
    );
    served.stop().await?;
    Ok(())
}
