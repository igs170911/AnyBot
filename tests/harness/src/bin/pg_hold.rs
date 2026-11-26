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
use serde as _;
use serde_json as _;
use sha2 as _;
use socket2 as _;
use sqlx as _;
use tokio_stream as _;
use toml as _;
use tracing as _;
use tracing_subscriber as _;
use url as _;

use std::path::PathBuf;

use test_harness::{
    pg_start, pg_stop, reap_leftover_tools, remove_tool_pid, test_dir, write_tool_pid,
};

#[tokio::main]
async fn main() -> Result<(), String> {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let label = argv.first().ok_or("missing data label")?;
    reap_leftover_tools()?;
    let dir = test_dir(label)?;
    let pid_file = write_tool_pid(&dir)?;
    let instance = pg_start(&dir).await?;
    let port = instance.port();
    let mut stdout = std::io::stdout();
    std::io::Write::write_fmt(&mut stdout, format_args!("pg-hold ready {port}\n"))
        .map_err(|e| e.to_string())?;
    let _flushed = std::io::Write::flush(&mut stdout);
    listen_for_stop().await.map_err(|err| {
        let mut stderr = std::io::stderr();
        let _written = std::io::Write::write_fmt(
            &mut stderr,
            format_args!("pg-hold stop signal failed: {err}\n"),
        );
        format!("pg-hold stop signal failed: {err}")
    })?;
    pg_stop(&instance).await?;
    remove_tool_pid(&pid_file)?;
    let _removed = std::fs::remove_dir_all(PathBuf::from(&dir));
    Ok(())
}

#[cfg(unix)]
async fn listen_for_stop() -> Result<(), std::io::Error> {
    let mut terminate = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
    tokio::select! {
        _ctrl = tokio::signal::ctrl_c() => Ok(()),
        Some(_sig) = terminate.recv() => Ok(()),
    }
}

#[cfg(windows)]
async fn listen_for_stop() -> Result<(), std::io::Error> {
    let mut ctrl_break = tokio::signal::windows::ctrl_break()?;
    let mut ctrl_close = tokio::signal::windows::ctrl_close()?;
    tokio::select! {
        _ctrl = tokio::signal::ctrl_c() => Ok(()),
        _break = ctrl_break.recv() => Ok(()),
        _close = ctrl_close.recv() => Ok(()),
    }
}

#[cfg(test)]
mod link_dev_deps {
    use anybot_store_server as _;
    use sad_web_push as _;
}
