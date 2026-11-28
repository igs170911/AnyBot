use anybot_foundation as _;
use anybot_server as _;
use serde as _;
use test_harness as _;
use uuid as _;

use axum::serve;

use test_harness::postgres::pg_stop;
use test_harness::reap_leftover_tools;
use test_harness::scratch::repo_root;
use test_harness::{remove_tool_pid, write_tool_pid};

use testenv::env::testenv_start;
use testenv::stop::wait_for_stop;

async fn run() -> Result<(), String> {
    reap_leftover_tools()?;
    let (state, router, listener) = testenv_start().await?;
    let pid_dir = repo_root().join("target").join("tests").join("testenv");
    let pid_file = write_tool_pid(&pid_dir)?;
    let report = serde_json::to_string(&state.ports).map_err(|e| e.to_string())?;
    let mut stdout = std::io::stdout();
    std::io::Write::write_fmt(&mut stdout, format_args!("testenv ready {report}\n"))
        .map_err(|e| e.to_string())?;
    let _flushed = std::io::Write::flush(&mut stdout);
    let (stop_tx, stop_rx) = tokio::sync::oneshot::channel();
    let served = serve(listener, router)
        .with_graceful_shutdown(async move {
            let result = wait_for_stop().await;
            let _sent = stop_tx.send(result);
        })
        .await;
    served.map_err(|e| e.to_string())?;
    pg_stop(&state.pg).await?;
    remove_tool_pid(&pid_file)?;
    stop_rx
        .await
        .map_err(|e| format!("stop result missing: {e}"))?
}

#[tokio::main]
async fn main() -> Result<(), String> {
    run().await
}
