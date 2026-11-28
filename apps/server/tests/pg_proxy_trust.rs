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
async fn t_safety_n11_2() -> Result<(), String> {
    let mut served = start("t_safety_n11_2_full").await?;
    trust_proxy(&mut served, None).await?;
    fail_times(&served, &Via::proxy(Some("9.9.9.9, 1.2.3.4")), 8, "chain").await?;
    let same_last = Via::proxy(Some("8.8.8.8, 1.2.3.4"));
    assert_eq!(
        right_password(&served, &same_last, "k-same-last").await?,
        429
    );
    let same_first = Via::proxy(Some("9.9.9.9, 5.5.5.5"));
    assert_eq!(
        right_password(&served, &same_first, "k-same-first").await?,
        200
    );
    assert_eq!(
        right_password(&served, &Via::proxy(None), "k-peer").await?,
        200
    );
    let overlong = "a".repeat(65);
    fail_times(&served, &Via::proxy(Some(&overlong)), 8, "long").await?;
    assert_eq!(
        right_password(&served, &Via::proxy(None), "k-peer-full").await?,
        429
    );
    let other_long = "b".repeat(65);
    let at_peer = Via::proxy(Some(&other_long));
    assert_eq!(
        right_password(&served, &at_peer, "k-other-long").await?,
        429
    );
    let longest_kept = "c".repeat(64);
    let kept = Via::proxy(Some(&longest_kept));
    assert_eq!(right_password(&served, &kept, "k-64").await?, 200);
    served.stop().await
}

#[tokio::test]
async fn t_safety_n11_3() -> Result<(), String> {
    let mut served = start("t_safety_n11_3_full").await?;
    trust_proxy(&mut served, Some(SECRET_VALUE)).await?;
    fail_times(&served, &Via::other(Some("1.2.3.4")), 8, "neighbor").await?;
    let neighbor = Via::other(Some("5.6.7.8"));
    assert_eq!(right_password(&served, &neighbor, "k-neighbor").await?, 429);
    fail_times(&served, &Via::proxy(Some("1.2.3.4")), 8, "nosecret").await?;
    let unsigned = Via::proxy(Some("5.6.7.8"));
    assert_eq!(right_password(&served, &unsigned, "k-unsigned").await?, 429);
    let signed = Via::proxy(Some("1.2.3.4")).with_secret();
    assert_eq!(right_password(&served, &signed, "k-signed").await?, 200);
    served.stop().await
}
