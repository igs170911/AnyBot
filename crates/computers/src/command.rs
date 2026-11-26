use std::path::Path;
use std::time::Duration;

use anybot_foundation::{Clock, SystemClock, timeout};
use anybot_protocol::ComputerFailure;
use anybot_trust::OpRejection;

fn command_parts(line: &str) -> (String, Vec<String>) {
    let mut parts = line.split_whitespace();
    let program = parts.next().unwrap_or_default().to_owned();
    let args: Vec<String> = parts.map(|p| (*p).to_owned()).collect();
    (program, args)
}

pub(crate) async fn run_line(
    line: &str,
    workdir: &Path,
    timeout_ms: u64,
) -> Result<(Option<i32>, String, u64), OpRejection> {
    use tokio::process::Command as TokioCommand;
    let (program, args) = command_parts(line);
    let child = TokioCommand::new(&program)
        .args(&args)
        .current_dir(workdir)
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| {
            OpRejection::from_failure(ComputerFailure::operation(format!(
                "cannot start the command: {e}"
            )))
        })?;
    let started = SystemClock.now_ms().millis();
    let limit = Duration::from_millis(timeout_ms);
    let finished = timeout(&SystemClock, limit, child.wait_with_output()).await;
    let Ok(Ok(output)) = finished else {
        return Ok((Some(124), String::new(), timeout_ms));
    };
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let elapsed = SystemClock.now_ms().millis().saturating_sub(started);
    Ok((output.status.code(), stdout, elapsed))
}
