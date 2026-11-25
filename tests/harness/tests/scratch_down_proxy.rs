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
use std::path::PathBuf;
use tokio_stream as _;
use toml as _;
use tracing as _;
use tracing_subscriber as _;
use url as _;

use test_harness::{
    Program, Running, net_attach, net_down, net_restore, net_start_raw, pg_start, pg_stop, test_dir,
};

#[tokio::test]
async fn scratch_down_proxy_server() -> Result<(), String> {
    test_harness::test_tracing();
    let dir = test_dir("scratch_down_proxy")?;
    let pg = pg_start(&dir).await?;
    let _created = pg.create_database("anybot_test").await;
    let (ctl, proxy_port) = net_start_raw().await?;
    let real = std::net::SocketAddr::from(([127, 0, 0, 1], pg.port()));
    let _attached = net_attach(&ctl, proxy_port, real).await;
    tracing::debug!("SCRATCH cut done");
    let _cut = net_down(&ctl, proxy_port).await;
    tracing::debug!("SCRATCH down done");
    let url = format!(
        "postgres://{}:{}@127.0.0.1:{}/anybot_test",
        pg.user(),
        pg.password().expose(),
        proxy_port
    );
    let bootstrap = dir.join("boot.conf");
    let _written =
        test_harness::scratch::write_file_with_mode(&bootstrap, &format!("{url}\n"), 0o600);
    let bin = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/debug/anybot-server");
    let program = Program::new(
        &bin.display().to_string(),
        vec!["--bootstrap".to_owned(), bootstrap.display().to_string()],
    )
    .with_timeout(std::time::Duration::from_secs(30));
    let mut running = Running::start(&program, &dir)?;
    tracing::debug!("SCRATCH first line read");
    let line = running.next_stdout_line()?;
    if !line.contains("retrying in 5 seconds") {
        return Err(format!("first line {line}"));
    }
    test_harness::sleep_real(std::time::Duration::from_secs(20)).await;
    tracing::debug!("SCRATCH slept done");
    let _restore = net_restore(&ctl, proxy_port).await;
    tracing::debug!("SCRATCH restore done");
    let mut listening = String::new();
    for _round in 0..14 {
        let line = running.next_stdout_line()?;
        if line.contains("listening on") {
            listening = line;
            break;
        }
    }
    let _killed = running.kill();
    let _stopped = pg_stop(&pg).await;
    if listening.is_empty() {
        return Err("no listening line".to_owned());
    }
    Ok(())
}
