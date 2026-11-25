use anybot_foundation as _;
use anybot_server as _;
use serde as _;
use test_harness as _;

use axum::serve;

use testenv::env::testenv_start;

async fn run() -> Result<(), String> {
    let (state, router, listener) = testenv_start().await?;
    let report = serde_json::to_string(&state.ports).map_err(|e| e.to_string())?;
    let mut stdout = std::io::stdout();
    std::io::Write::write_fmt(&mut stdout, format_args!("testenv ready {report}\n"))
        .map_err(|e| e.to_string())?;
    let served = serve(listener, router).await;
    served.map_err(|e| e.to_string())?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), String> {
    run().await
}
