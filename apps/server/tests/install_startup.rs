use std::process::ExitCode;

use tokio::net::TcpListener;

use anybot_app_server::{bind_plan, help_text, parse_launch, startup_plan};

use anybot_foundation as _;
use anybot_server as _;
use axum as _;
use clap as _;

use test_harness::{Program, Running, http_get, run_program, test_dir, write_file_with_mode};

const GOOD_BOOTSTRAP: &str =
    "postgresql://anybot:fixture-not-a-secret@localhost:5432/anybot_test\n";

fn server_program(args: Vec<String>) -> Program {
    Program::new(env!("CARGO_BIN_EXE_anybot-server"), args)
}

fn bootstrap_args(path: &std::path::Path) -> Vec<String> {
    vec!["--bootstrap".to_owned(), path.display().to_string()]
}

fn scratch_bootstrap(test: &str, mode: u32) -> Result<std::path::PathBuf, String> {
    let dir = test_dir(test)?;
    let path = dir.join("boot.conf");
    write_file_with_mode(&path, GOOD_BOOTSTRAP, mode)?;
    Ok(path)
}

#[tokio::test]
async fn t_install_n22_1() -> Result<(), String> {
    let path = scratch_bootstrap("install_n22_1", 0o600)?;
    let dir = path.parent().ok_or("啟動檔要有資料夾")?.to_path_buf();
    let mut running = Running::start(&server_program(bootstrap_args(&path)), &dir)?;
    let first_line = running.next_stdout_line()?;
    assert_eq!(
        first_line.trim(),
        format!(
            "AnyBot 伺服器 v{}：聽 127.0.0.1:8787、[::1]:8787（資料庫 localhost/anybot_test）",
            env!("CARGO_PKG_VERSION")
        )
    );
    assert!(!first_line.contains("fixture-not-a-secret"));
    let _result = running.kill();
    Ok(())
}

#[tokio::test]
async fn t_install_n22_2() -> Result<(), String> {
    let dir = test_dir("install_n22_2")?;
    let outcome = run_program(&server_program(vec!["--help".to_owned()]), &dir)?;
    assert_eq!(outcome.code, 0);
    assert!(outcome.stdout.contains("reset-password"));
    assert!(outcome.stdout.contains("logs"));
    assert!(outcome.stdout.contains("update-bot-computers"));
    assert!(outcome.stdout.contains("migrate-old"));
    assert!(outcome.stdout.contains("--bootstrap"));
    assert!(outcome.stdout.contains("--version"));
    Ok(())
}

async fn hold_port_8787() -> Result<TcpListener, String> {
    for _attempt in 0..40 {
        match TcpListener::bind("127.0.0.1:8787").await {
            Ok(listener) => return Ok(listener),
            Err(err) if err.kind() == std::io::ErrorKind::AddrInUse => {
                tokio::time::sleep(std::time::Duration::from_millis(250)).await;
            }
            Err(err) => return Err(err.to_string()),
        }
    }
    Err("佔住 8787 失敗：超過等待時間".to_owned())
}

#[tokio::test]
async fn t_install_n22_3() -> Result<(), String> {
    let held = hold_port_8787().await?;
    let dir = test_dir("install_n22_3")?;
    let path = scratch_bootstrap("install_n22_3", 0o600)?;
    let outcome = run_program(&server_program(bootstrap_args(&path)), &dir)?;
    assert_eq!(outcome.code, 1);
    assert!(outcome.stderr.contains("無法使用連接埠 8787"));
    drop(held);
    Ok(())
}

#[tokio::test]
async fn t_install_n22_4() -> Result<(), String> {
    let dir = test_dir("install_n22_4")?;
    let no_file = run_program(&server_program(vec![]), &dir)?;
    assert_eq!(no_file.code, 2);
    assert!(no_file.stderr.contains("找不到啟動檔"));
    assert!(no_file.stderr.contains("--bootstrap"));
    let path = dir.join("x.conf");
    let unknown = run_program(
        &server_program(vec![
            "--bootstrap".to_owned(),
            path.display().to_string(),
            "--port".to_owned(),
            "1".to_owned(),
        ]),
        &dir,
    )?;
    assert_eq!(unknown.code, 2);
    Ok(())
}

#[tokio::test]
async fn t_server_n14_2() -> Result<(), String> {
    let path = scratch_bootstrap("server_n14_2", 0o644)?;
    let dir = path.parent().unwrap().to_path_buf();
    let outcome = run_program(&server_program(bootstrap_args(&path)), &dir)?;
    assert_ne!(outcome.code, 0);
    assert!(outcome.stderr.contains("啟動檔的權限太開放"));
    Ok(())
}

#[test]
fn startup_line_reports_both_addresses_without_password() {
    let path = scratch_bootstrap("unit_startup_line", 0o600).unwrap();
    let args = parse_launch(&bootstrap_args(&path)).unwrap();
    let startup = startup_plan(&args, "9.9.9").unwrap();
    assert_eq!(
        startup.startup_line,
        "AnyBot 伺服器 v9.9.9：聽 127.0.0.1:8787、[::1]:8787（資料庫 localhost/anybot_test）"
    );
    assert!(!startup.startup_line.contains("fixture-not-a-secret"));
}

#[tokio::test]
async fn bind_plan_says_port_unusable_when_port_is_taken() {
    let held = hold_port_8787().await.unwrap();
    let failure = bind_plan(8787).await.unwrap_err();
    assert_eq!(failure.code, ExitCode::from(1));
    assert!(failure.message.contains("無法使用連接埠 8787"));
    drop(held);
}

#[test]
fn startup_plan_says_bootstrap_flag_when_file_is_missing() {
    let args = parse_launch(&[]).unwrap();
    let failure = startup_plan(&args, "9.9.9").unwrap_err();
    assert_eq!(failure.code, ExitCode::from(2));
    assert!(failure.message.contains("找不到啟動檔"));
    assert!(failure.message.contains("--bootstrap"));
}

#[test]
fn parse_launch_rejects_unknown_flag_with_exit_code_two() {
    let failure = parse_launch(&[
        "--bootstrap".to_owned(),
        "x.conf".to_owned(),
        "--port".to_owned(),
        "1".to_owned(),
    ])
    .unwrap_err();

    assert_eq!(failure.code, ExitCode::from(2));
}

#[test]
fn help_text_lists_all_subcommands_and_flags() {
    let help = help_text();
    assert!(help.contains("reset-password"));
    assert!(help.contains("--bootstrap"));
    assert!(help.contains("--version"));
}

#[test]
fn startup_plan_rejects_world_readable_bootstrap() {
    let path = scratch_bootstrap("unit_permissions", 0o644).unwrap();
    let args = parse_launch(&bootstrap_args(&path)).unwrap();
    let failure = startup_plan(&args, "9.9.9").unwrap_err();
    assert_ne!(failure.code, ExitCode::SUCCESS);
    assert!(failure.message.contains("啟動檔的權限太開放"));
}

#[test]
fn help_and_version_print_to_stdout_with_zero_exit() {
    let dir = test_dir("unit_help_stdout").unwrap();
    let help = run_program(&server_program(vec!["--help".to_owned()]), &dir).unwrap();
    assert_eq!(help.code, 0);
    assert!(!help.stderr.contains("Usage:"));
    let version = run_program(&server_program(vec!["--version".to_owned()]), &dir).unwrap();
    assert_eq!(version.code, 0);
    assert!(version.stdout.contains("anybot-server"));
}

async fn start_health_server(
    path: &std::path::Path,
    dir: &std::path::Path,
) -> Result<Running, String> {
    for _attempt in 0..40 {
        let mut running = Running::start(&server_program(bootstrap_args(path)), dir)?;
        let line = running.next_stdout_line()?;
        assert!(line.contains("聽 127.0.0.1:8787"), "{line}");
        if http_get(8787, "/v1/health").is_ok() {
            return Ok(running);
        }
        let outcome = running.kill()?;
        assert!(
            outcome.stderr.contains("無法使用連接埠 8787"),
            "{}",
            outcome.stderr
        );
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    }
    Err("8787 一直無法使用".to_owned())
}

#[tokio::test]
async fn health_endpoint_answers_on_the_started_server() -> Result<(), String> {
    let path = scratch_bootstrap("unit_health", 0o600).unwrap();
    let dir = path.parent().unwrap().to_path_buf();
    let mut running = start_health_server(&path, &dir).await?;
    let (code, _body) = http_get(8787, "/v1/health")?;
    assert_eq!(code, 200);
    let _result = running.kill()?;
    Ok(())
}
