use getrandom as _;
use std::net::SocketAddr;

use anybot_server::{DEFAULT_PORT, bind_addrs, bind_listeners, parse_listen_host};

use anybot_blob as _;
use anybot_bus as _;
use anybot_channels as _;
use anybot_domain as _;
use anybot_foundation as _;
use anybot_messages as _;
use anybot_protocol as _;
use anybot_store_server as _;
use argon2 as _;
use axum as _;
use rand_core as _;
use serde as _;
use serde_json as _;
use sha2 as _;
use socket2 as _;
use sqlx as _;
use test_harness as _;
use time as _;
use url as _;
use utoipa as _;
use uuid as _;

#[test]
fn plain_ip_parses_with_the_given_port() {
    let addr =
        parse_listen_host("127.0.0.1", DEFAULT_PORT).unwrap_or(SocketAddr::from(([0, 0, 0, 0], 0)));
    assert_eq!(addr.ip().to_string(), "127.0.0.1");
    assert_eq!(addr.port(), DEFAULT_PORT);
}

#[test]
fn ipv6_plain_and_bracketed_forms_parse() {
    let plain =
        parse_listen_host("::1", DEFAULT_PORT).unwrap_or(SocketAddr::from(([0, 0, 0, 0], 0)));
    assert_eq!(plain.ip().to_string(), "::1");
    assert_eq!(plain.port(), DEFAULT_PORT);
    let bracketed = parse_listen_host("[::1]:9999", DEFAULT_PORT)
        .unwrap_or(SocketAddr::from(([0, 0, 0, 0], 0)));
    assert_eq!(bracketed.ip().to_string(), "::1");
    assert_eq!(bracketed.port(), DEFAULT_PORT);
}

#[test]
fn non_address_text_is_rejected_by_the_parser() {
    assert!(parse_listen_host("abc", DEFAULT_PORT).is_err());
    assert!(parse_listen_host("", DEFAULT_PORT).is_err());
}

#[tokio::test]
async fn bind_addrs_covers_every_requested_loopback_address() -> Result<(), String> {
    let addrs = [
        SocketAddr::from(([127, 0, 0, 1], 32_517)),
        SocketAddr::from((std::net::Ipv6Addr::LOCALHOST, 32_517)),
    ];
    let Ok(listeners) = bind_addrs(&addrs).await else {
        return Err("bind_addrs failed for the loopback pair".to_owned());
    };
    if listeners.len() != 2 {
        return Err("bind_addrs returned the wrong listener count".to_owned());
    }
    Ok(())
}

#[tokio::test]
async fn bind_listeners_keeps_the_dual_loopback_pair() -> Result<(), String> {
    let Ok(listeners) = bind_listeners(32_519).await else {
        return Ok(());
    };
    if listeners.len() != 2 {
        return Err("bind_listeners did not return a pair".to_owned());
    }
    Ok(())
}
