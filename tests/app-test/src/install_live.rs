use anybot_messages::Params;
use test_harness::{Outcome, Running, http_get};

use crate::{ServerDb, ServerProgram};

#[must_use]
pub fn bootstrap_args(path: &std::path::Path) -> Vec<String> {
    vec!["--bootstrap".to_owned(), path.display().to_string()]
}

#[must_use]
pub fn startup_params(version: &str, port: u16) -> Params {
    Params::new()
        .with_text("version", version)
        .with_text("addresses", &format!("127.0.0.1:{port}, [::1]:{port}"))
        .with_text("db_host", "127.0.0.1")
        .with_text("db_name", "anybot_test")
}

pub async fn wait_health(db: &ServerDb) -> Result<(), String> {
    for _attempt in 0..40 {
        let (code, _body) = http_get(db.listen_port, "/v1/health").await?;
        if code == 200 {
            return Ok(());
        }
        test_harness::sleep_real(std::time::Duration::from_millis(250)).await;
    }
    Err("health endpoint never answered 200".to_owned())
}

pub fn start_running(
    bin: &ServerProgram,
    db: &ServerDb,
    expected: &str,
) -> Result<Running, String> {
    bin.start_running(db, expected)
}

pub async fn start_live(
    bin: &ServerProgram,
    db: &ServerDb,
    expected: &str,
) -> Result<Running, String> {
    let running = start_running(bin, db, expected)?;
    match wait_health(db).await {
        Ok(()) => Ok(running),
        Err(msg) => Err(msg),
    }
}

pub async fn stop_live(mut running: Running) -> Result<Running, String> {
    running.terminate()?;
    finish_running(&mut running).await.map(|_outcome| running)
}

pub async fn finish_running(running: &mut Running) -> Result<Outcome, String> {
    for _tick in 0..80 {
        match running.child().try_wait() {
            Ok(Some(_status)) => return running.finish(),
            Ok(None) => {}
            Err(err) => return Err(err.to_string()),
        }
        test_harness::sleep_real(std::time::Duration::from_millis(250)).await;
    }
    running.kill()
}

pub async fn get_status(db: &ServerDb) -> Result<String, String> {
    let (code, body) = http_get(db.listen_port, "/v1/server/status").await?;
    if code != 200 {
        return Err(format!("status code {code}"));
    }
    Ok(body)
}

pub fn public_url_check(body: &str) -> Result<(), String> {
    if body.contains("https://next.example/") {
        return Ok(());
    }
    Err(format!("status body misses public url: {body}"))
}

pub fn check_both(
    body: Result<String, String>,
    stopped_now: Result<(), String>,
    stopped_db: Result<(), String>,
) -> Result<(), String> {
    match body {
        Ok(text) => match public_url_check(&text) {
            Ok(()) => match stopped_now {
                Ok(()) => stopped_db,
                Err(msg) => Err(msg),
            },
            Err(msg) => Err(msg),
        },
        Err(msg) => Err(msg),
    }
}

pub fn check_second(second: &Outcome, port: u16) -> Result<(), String> {
    if second.code != 1 {
        return Err(format!("second server exit {}", second.code));
    }
    if !second.stdout.is_empty() {
        return Err(format!("second server printed startup output {second:?}"));
    }
    let want = format!("Cannot use port {port}:");
    if !second.stderr.contains(&want) {
        return Err(format!("{second:?} misses the port message"));
    }
    Ok(())
}
