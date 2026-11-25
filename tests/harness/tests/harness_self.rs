use aes_gcm as _;
use anybot_foundation as _;
use axum as _;
use base64 as _;
use bytes as _;
use getrandom as _;
use hkdf as _;
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

extern crate libc as _;
extern crate reqwest as _;

use std::sync::Mutex;

use test_harness::{
    Program, child_is_dead, http_get, run_program, run_program_with, test_dir, write_file_with_mode,
};

#[test]
fn harness_run_program_captures_stdout_and_exit_code() {
    let dir = test_dir("harness-run").unwrap();
    let program = Program::new("/bin/echo", vec!["hi-there".to_owned()]);
    let outcome = run_program(&program, &dir).unwrap();
    assert_eq!(outcome.code, 0);
    assert_eq!(outcome.stdout.trim(), "hi-there");
    assert_eq!(outcome.stderr, "");
}

#[test]
fn harness_run_program_captures_nonzero_exit_and_stderr() {
    let dir = test_dir("harness-fail").unwrap();
    let program = Program::new(
        "/bin/sh",
        vec!["-c".to_owned(), "echo oops >&2; exit 3".to_owned()],
    );
    let outcome = run_program(&program, &dir).unwrap();
    assert_eq!(outcome.code, 3);
    assert!(outcome.stderr.contains("oops"));
}

#[test]
fn harness_running_pid_matches_child_and_kill_stops_it() {
    let dir = test_dir("harness-kill").unwrap();
    let program = Program::new("/bin/sleep", vec!["60".to_owned()]);
    let mut running = test_harness::Running::start(&program, &dir).unwrap();
    assert!(running.pid() > 0);
    let outcome = running.kill().unwrap();
    assert_ne!(outcome.code, 0);
}

#[test]
fn harness_test_dir_is_under_target_and_is_fresh() {
    let dir = test_dir("harness-fresh").unwrap();
    assert!(dir.starts_with(test_harness::scratch::repo_root().join("target")));
    write_file_with_mode(&dir.join("a.txt"), "x", 0o600).unwrap();
    let second = test_dir("harness-fresh").unwrap();
    assert_eq!(dir, second);
    assert!(!second.join("a.txt").exists());
}

#[cfg(unix)]
#[test]
fn harness_write_file_with_mode_sets_exact_permissions() {
    use std::os::unix::fs::PermissionsExt;
    let dir = test_dir("harness-mode").unwrap();
    let path = dir.join("boot.conf");
    write_file_with_mode(&path, "postgresql://u:p@h/db\n", 0o600).unwrap();
    let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600);
}

#[test]
fn harness_run_program_times_out_and_kills_child() {
    let dir = test_dir("harness-timeout").unwrap();
    let program = Program::new("/bin/sleep", vec!["600".to_owned()])
        .with_timeout(std::time::Duration::from_secs(2));
    let started = std::time::Instant::now();
    let spawned = Mutex::new(Vec::new());
    let result = run_program_with(&program, &dir, |child| {
        spawned.lock().unwrap().push(child.id());
        Ok(())
    });
    let elapsed = started.elapsed();
    let err = result.unwrap_err();
    assert!(err.contains("timed out"), "{err}");
    assert!(elapsed < std::time::Duration::from_secs(6), "{elapsed:?}");
    assert!(err.contains("to exit"), "{err}");
    let pid = spawned.lock().unwrap().first().copied().unwrap();
    assert!(err.contains(&format!("PID {pid}")), "{err}");
    let alive = std::process::Command::new("/bin/kill")
        .args(["-0", &pid.to_string()])
        .status()
        .unwrap()
        .success();
    assert!(!alive, "child PID {pid} still alive after timeout");
}

#[test]
fn harness_run_program_drains_one_megabyte_of_stderr() {
    let dir = test_dir("harness-big-stderr").unwrap();
    let program = Program::new(
        "/bin/sh",
        vec![
            "-c".to_owned(),
            "i=0; while [ $i -lt 64 ]; do seq 16384 | tr '0' x >&2; i=$((i+1)); done; exit 7"
                .to_owned(),
        ],
    );
    let outcome = run_program(&program, &dir).unwrap();
    assert_eq!(outcome.code, 7);
    assert!(
        outcome.stderr.len() >= 1_000_000,
        "{}",
        outcome.stderr.len()
    );
}

#[test]
fn harness_running_line_wait_times_out_and_kills_child() {
    let dir = test_dir("harness-line-timeout").unwrap();
    let program = Program::new("/bin/sleep", vec!["600".to_owned()])
        .with_timeout(std::time::Duration::from_secs(2));
    let mut running = test_harness::Running::start(&program, &dir).unwrap();
    let started = std::time::Instant::now();
    let err = running.next_stdout_line().unwrap_err();
    assert!(err.contains("timed out"), "{err}");
    assert!(
        started.elapsed() < std::time::Duration::from_secs(6),
        "{:?}",
        started.elapsed()
    );
    let dead = child_is_dead(running.child()).unwrap();
    assert!(dead, "child still alive after line timeout");
}

#[tokio::test]
async fn harness_http_get_reports_connection_failure_without_crashing() {
    let result = http_get(1, "/v1/health").await;
    assert!(result.is_err());
}
