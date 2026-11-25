use aes_gcm as _;
use anybot_foundation as _;
use axum as _;
use base64 as _;
use bytes as _;
use getrandom as _;
use hkdf as _;
use libc as _;
use p256 as _;
use postgresql_embedded as _;
use reqwest as _;
use sad_web_push as _;
use serde as _;
use serde_json as _;
use sha2 as _;
use socket2 as _;
use sqlx as _;
use tokio_stream as _;
use toml as _;
use url as _;

use std::time::Duration;

use test_harness::netctl::{
    NetMode, net_cut, net_cut_target, net_mode, net_restore, net_restore_target, net_slow,
    net_start,
};

#[tokio::test]
async fn netctl_modes_round_trip() {
    let (ctl, port) = net_start().await.unwrap();
    assert_eq!(net_mode(&ctl, port).await, NetMode::Open);
    net_cut(&ctl, port).await.unwrap();
    assert_eq!(net_mode(&ctl, port).await, NetMode::Cut);
    net_slow(&ctl, port, Duration::from_millis(1000))
        .await
        .unwrap();
    assert_eq!(
        net_mode(&ctl, port).await,
        NetMode::Slow {
            connect: Duration::from_millis(1000),
            first_byte: Duration::from_millis(1000),
            byte_gap: Duration::from_millis(1000),
        }
    );
    net_restore(&ctl, port).await.unwrap();
    assert_eq!(net_mode(&ctl, port).await, NetMode::Open);
}

#[tokio::test]
async fn netctl_global_cut_overrides_port() {
    let (ctl, port) = net_start().await.unwrap();
    net_cut(&ctl, port).await.unwrap();
    net_cut_target(&ctl, port).await.unwrap();
    assert_eq!(net_mode(&ctl, port).await, NetMode::Cut);
    net_restore_target(&ctl, port).await.unwrap();
    assert_eq!(net_mode(&ctl, port).await, NetMode::Open);
    net_restore(&ctl, port).await.unwrap();
    assert_eq!(net_mode(&ctl, port).await, NetMode::Open);
}
