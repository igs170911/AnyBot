use anybot_app_server as _;
use anybot_domain as _;
use anybot_foundation as _;
use anybot_messages as _;
use anybot_server_test as _;
use anybot_store_server as _;
use axum as _;
use clap as _;
use rpassword as _;
use sqlx as _;
use test_harness as _;
use tokio as _;
use tracing as _;

mod pg_proxy_shared;
use pg_proxy_shared::{SECRET_VALUE, Via, fail_times, right_password, start, trust_proxy};

#[tokio::test]
async fn t_server_n1_2() -> Result<(), String> {
    let mut served = start("t_server_n1_2_full").await?;
    trust_proxy(&mut served, None).await?;
    let chain = Some("6.6.6.6, 1.2.3.4");
    fail_times(&served, &Via::proxy(chain), 8, "proxy").await?;
    let same_last = Via::proxy(Some("7.7.7.7, 1.2.3.4"));
    assert_eq!(
        right_password(&served, &same_last, "k-same-last").await?,
        429
    );
    let same_first = Via::proxy(Some("6.6.6.6, 2.2.2.2"));
    assert_eq!(
        right_password(&served, &same_first, "k-same-first").await?,
        200
    );
    assert_eq!(
        right_password(&served, &Via::other(chain), "k-other").await?,
        200
    );
    fail_times(&served, &Via::other(chain), 8, "other").await?;
    assert_eq!(
        right_password(&served, &Via::other(None), "k-other-peer").await?,
        429
    );
    served.stop().await
}

#[tokio::test]
async fn t_server_n1_3() -> Result<(), String> {
    let mut served = start("t_server_n1_3_full").await?;
    trust_proxy(&mut served, Some(SECRET_VALUE)).await?;
    fail_times(&served, &Via::proxy(Some("1.2.3.4")), 8, "unsigned").await?;
    let unsigned = Via::proxy(Some("5.6.7.8"));
    assert_eq!(right_password(&served, &unsigned, "k-unsigned").await?, 429);
    let signed = Via::proxy(Some("1.2.3.4")).with_secret();
    assert_eq!(right_password(&served, &signed, "k-signed").await?, 200);
    served.stop().await
}
