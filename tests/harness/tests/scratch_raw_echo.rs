use aes_gcm as _;
use anybot_foundation as _;
use anybot_store_server as _;
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
use tracing as _;
use tracing_subscriber as _;
use tracing_subscriber as _;
use url as _;

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use anybot_foundation::ServerBootstrap;
use test_harness::{
    net_attach, net_down, net_restore, net_start_raw, pg_start, pg_stop, test_dir, timeout_real,
};

async fn echo_listener() -> Result<SocketAddr, String> {
    let Ok(listener) = TcpListener::bind(("127.0.0.1", 0)).await else {
        return Err("echo bind failed".to_owned());
    };
    let Ok(addr) = listener.local_addr() else {
        return Err("echo local addr failed".to_owned());
    };
    tokio::spawn(async move {
        loop {
            let Ok((mut stream, _peer)) = listener.accept().await else {
                continue;
            };
            tokio::spawn(async move {
                let mut buf = [0u8; 64];
                loop {
                    let Ok(n) = stream.read(&mut buf).await else {
                        return;
                    };
                    if n == 0 {
                        return;
                    }
                    let Some(part) = buf.get(..n) else {
                        return;
                    };
                    if stream.write_all(part).await.is_err() {
                        return;
                    }
                    if stream.flush().await.is_err() {
                        return;
                    }
                }
            });
        }
    });
    Ok(addr)
}

async fn round_trip(proxy_port: u16) -> Result<String, String> {
    let connect = timeout_real(
        tokio::net::TcpStream::connect(("127.0.0.1", proxy_port)),
        Duration::from_secs(3),
    )
    .await
    .map_err(|e| format!("connect: {e}"))?
    .map_err(|e| format!("connect failed {e}"))?;
    let mut stream = connect;
    let _written = stream.write_all(b"ping").await.map_err(|e| e.to_string())?;
    let _flushed = stream.flush().await.map_err(|e| e.to_string())?;
    let mut buf = [0u8; 4];
    let _read = timeout_real(stream.read_exact(&mut buf), Duration::from_secs(3))
        .await
        .map_err(|e| format!("read: {e}"))?
        .map_err(|e| format!("read failed {e}"))?;
    Ok(String::from_utf8_lossy(&buf).to_string())
}

#[tokio::test]
async fn netctl_raw_probe_open_down_restore() -> Result<(), String> {
    let dir = test_dir("scratch_raw_probe")?;
    let pg = pg_start(&dir).await?;
    let _seed = pg.create_database("anybot_seed").await;
    let url = pg.url_for("anybot_probe");
    let seed_url = pg.url_for("anybot_seed");
    let create = sqlx::postgres::PgPoolOptions::new()
        .acquire_timeout(Duration::from_secs(30))
        .connect(&seed_url)
        .await;
    let create = match create {
        Ok(pool) => pool,
        Err(e) => return Err(format!("create pool {e}")),
    };
    let _made = sqlx::query("CREATE DATABASE anybot_probe")
        .execute(&create)
        .await
        .map_err(|e| e.to_string())?;
    let boot_path = dir.join("boot.conf");
    let _written = test_harness::write_file_with_mode(&boot_path, &format!("{url}\n"), 0o600);
    let bootstrap = ServerBootstrap::read(&boot_path).map_err(|e| e.to_string())?;
    let (ctl, proxy_port) = net_start_raw().await?;
    let real = SocketAddr::from(([127, 0, 0, 1], pg.port()));
    let _attached = net_attach(&ctl, proxy_port, real).await;
    let _cut = net_down(&ctl, proxy_port).await;
    test_harness::sleep_real(Duration::from_secs(20)).await;
    let _restored = net_restore(&ctl, proxy_port).await;
    for _round in 0..10 {
        test_harness::sleep_real(Duration::from_secs(2)).await;
        let gate = anybot_store_server::probe_pool(&bootstrap);
        let checked = anybot_store_server::probe(&gate).await;
        match checked {
            Ok(_one) => {
                let _stop = pg_stop(&pg).await;
                return Ok(());
            }
            Err(e) => std::eprintln!("probe fail {e}"),
        }
    }
    let _stop = pg_stop(&pg).await;
    Err("probe never succeeded through raw proxy".to_owned())
}

#[tokio::test]
async fn netctl_raw_echo_open_down_restore() -> Result<(), String> {
    let real = echo_listener().await?;
    let (ctl, proxy_port) = net_start_raw().await?;
    let _attached = net_attach(&ctl, proxy_port, real).await;
    let open = round_trip(proxy_port).await;
    let _open = open?;
    let _cut = net_down(&ctl, proxy_port).await;
    test_harness::sleep_real(Duration::from_secs(20)).await;
    let blocked = round_trip(proxy_port).await;
    if blocked.is_ok() {
        return Err("down mode forwarded data".to_owned());
    }
    let _restored = net_restore(&ctl, proxy_port).await;
    let mut back = round_trip(proxy_port).await;
    for _round in 0..40 {
        if back.is_ok() {
            break;
        }
        test_harness::sleep_real(Duration::from_millis(100)).await;
        back = round_trip(proxy_port).await;
    }
    let back = match back {
        Ok(text) => text,
        Err(msg) => return Err(format!("restore round trip: {msg}")),
    };
    let _ctl = Arc::clone(&ctl);
    if back != "ping" {
        return Err(format!("echo returned {back}"));
    }
    Ok(())
}
