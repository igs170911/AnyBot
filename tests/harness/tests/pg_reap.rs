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
use url as _;

use std::time::Duration;

use test_harness::postgres::pg_ready;
use test_harness::scratch::{repo_root, test_dir};
use test_harness::{Program, Running};

fn pid_alive(pid: u32) -> bool {
    std::process::Command::new("/bin/kill")
        .args(["-0", &pid.to_string()])
        .status()
        .is_ok_and(|code| code.success())
}

fn postmaster_pid(data_dir: &std::path::Path) -> Option<u32> {
    let text = std::fs::read_to_string(data_dir.join("postmaster.pid")).ok()?;
    text.lines().next()?.trim().parse::<u32>().ok()
}

#[tokio::test]
async fn leftover_postgres_stops_when_its_recorded_pid_is_signaled() {
    let dir = test_dir("pg-reap-signal").unwrap();
    let bin = env!("CARGO_BIN_EXE_pg_hold");
    let program =
        Program::new(bin, vec!["pg-reap-hold".to_owned()]).with_timeout(Duration::from_secs(40));
    let mut running = Running::start(&program, &dir).unwrap();
    let line = running.next_stdout_line().unwrap();
    assert!(line.contains("pg-hold ready"), "{line}");
    let port: u16 = line
        .split_whitespace()
        .next_back()
        .and_then(|text| text.trim().parse().ok())
        .expect("port");
    let hold_dir = repo_root()
        .join("target")
        .join("tests")
        .join("pg-reap-hold");
    let postmaster = postmaster_pid(&hold_dir.join("data")).expect("postmaster pid");
    assert!(
        pg_ready(&hold_dir, port),
        "postgres did not answer on {port}"
    );
    let pid = running.pid();
    running.terminate().unwrap();
    let outcome = running.finish().unwrap();
    assert_eq!(outcome.code, 0, "{outcome:?}");
    assert!(!pid_alive(pid), "holder PID {pid} is still alive");
    assert!(
        !pid_alive(postmaster),
        "postmaster PID {postmaster} is still alive"
    );
    assert!(!pg_ready(&hold_dir, port), "port {port} still answers");
}

#[test]
fn leftover_tool_pid_stops_only_when_the_process_is_this_workspace() {
    let dir = test_dir("pg-reap-tool").unwrap();
    let bin = env!("CARGO_BIN_EXE_pg_hold");
    let program =
        Program::new(bin, vec!["pg-reap-scan".to_owned()]).with_timeout(Duration::from_secs(45));
    let mut running = Running::start(&program, &dir).unwrap();
    let line = running.next_stdout_line().unwrap();
    assert!(line.contains("pg-hold ready"), "{line}");
    let pid = running.pid();
    let hold_dir = repo_root()
        .join("target")
        .join("tests")
        .join("pg-reap-scan");
    let marker = hold_dir.join("tool.pid");
    let body = format!("{pid}\n{bin}\n{}\n", repo_root().display());
    std::fs::write(&marker, &body).unwrap();
    let outside = repo_root().join("target").join("not-tests");
    std::fs::create_dir_all(&outside).unwrap();
    std::fs::write(outside.join("tool.pid"), &body).unwrap();
    let reaped = test_harness::reap_except(Some(&dir));
    assert!(reaped.is_ok(), "{reaped:?}");
    assert!(
        pid_alive(pid),
        "scan stopped a pid recorded outside target/tests"
    );
    let _removed_outside = std::fs::remove_dir_all(&outside);
    let elsewhere = repo_root()
        .join("target")
        .join("tests")
        .join("pg-reap-elsewhere");
    std::fs::create_dir_all(&elsewhere).unwrap();
    std::fs::write(elsewhere.join("tool.pid"), &body).unwrap();
    let reaped = test_harness::reap_except(Some(&dir));
    assert!(reaped.is_ok(), "{reaped:?}");
    let outcome = running.finish().unwrap();
    assert_eq!(outcome.code, 0, "{outcome:?}");
    assert!(
        !pid_alive(pid),
        "workspace child PID {pid} survived the scan"
    );
    let _removed = std::fs::remove_dir_all(elsewhere);
}

#[test]
fn term_signal_stops_a_child_that_handles_it() {
    let dir = test_dir("pg-reap-term").unwrap();
    let script = "trap 'exit 0' TERM; echo ready; while true; do sleep 1; done";
    let program = Program::new("/bin/sh", vec!["-c".to_owned(), script.to_owned()])
        .with_timeout(Duration::from_secs(8));
    let mut running = Running::start(&program, &dir).unwrap();
    let line = running.next_stdout_line().unwrap();
    assert!(line.contains("ready"), "{line}");
    let pid = running.pid();
    running.terminate().unwrap();
    let outcome = running.finish().unwrap();
    assert_eq!(outcome.code, 0, "{outcome:?}");
    assert!(!pid_alive(pid), "signaled child PID {pid} is still alive");
}
