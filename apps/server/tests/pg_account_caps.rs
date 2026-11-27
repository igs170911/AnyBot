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

use std::time::Duration;

use anybot_app_test::serve::serve_start;
use anybot_server_test::computers_support::{
    RedeemAttempt, api_get, computer_rows, login_as, make_code_fast, reauth, redeem,
};
use anybot_server_test::make_admin;

#[tokio::test]
async fn t_server_n2_2() -> Result<(), String> {
    let served = serve_start("t_server_n2_2").await?;
    let first_phone = make_admin(&served).await?;
    let mut phones = vec![first_phone];
    for number in 2..=51 {
        let _woken = served.clock.advance(Duration::from_secs(1));
        let logged = login_as(&served, "phone", "bearer", &format!("phone-{number}")).await?;
        assert_eq!(logged.status, 200, "phone {number}: {}", logged.json);
        phones.push(logged.bearer()?);
    }
    let oldest = phones.first().ok_or("no phones")?;
    let newest = phones.last().ok_or("no phones")?;
    let second = phones.get(1).ok_or("no second phone")?;
    let gone = api_get(&served, oldest, "/v1/computers").await?;
    assert_eq!(
        gone.status, 401,
        "the longest unused phone still works: {}",
        gone.json
    );
    for kept in [second, newest] {
        let alive = api_get(&served, kept, "/v1/computers").await?;
        assert_eq!(alive.status, 200, "{}", alive.json);
    }
    let entered = reauth(&served, newest, "alicepassword", "k-reauth").await?;
    assert_eq!(entered.status, 200, "{}", entered.json);
    let mut computers = Vec::new();
    for number in 1..=101 {
        let _woken = served.clock.advance(Duration::from_secs(1));
        let made = make_code_fast(&served, newest, &format!("code-{number}")).await?;
        assert_eq!(made.status, 200, "code {number}: {}", made.json);
        let code = made.field_str("code")?;
        let key = format!("pair-{number}");
        let name = format!("machine-{number}");
        let attempt = RedeemAttempt {
            code: &code,
            name: &name,
            key: &key,
            address: None,
        };
        let paired = redeem(&served, &attempt).await?;
        assert_eq!(paired.status, 200, "pair {number}: {}", paired.json);
        computers.push(paired.bearer()?);
    }
    let rows = computer_rows(&served, newest).await?;
    assert_eq!(rows.len(), 100);
    let dropped = computers.first().ok_or("no computers")?;
    let refused = api_get(&served, dropped, "/v1/computers").await?;
    assert_eq!(refused.status, 401, "{}", refused.json);
    let last = computers.last().ok_or("no computers")?;
    let alive = api_get(&served, last, "/v1/computers").await?;
    assert_eq!(alive.status, 200, "{}", alive.json);
    served.stop().await?;
    Ok(())
}
