use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::ExitCode;

use tokio::net::TcpListener;

use anybot_app_server::{bind_plan, parse_launch, startup_plan};

use anybot_foundation as _;
use anybot_server as _;
use axum as _;
use clap as _;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/bootstrap")
        .join(name)
}

fn set_mode(path: &std::path::Path, mode: u32) -> Result<(), String> {
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).map_err(|e| e.to_string())
}

fn bootstrap_args(path: &std::path::Path) -> Vec<String> {
    vec!["--bootstrap".to_owned(), path.display().to_string()]
}

#[tokio::test]
async fn t_install_n22_1() -> Result<(), String> {
    let path = fixture("server.conf");
    set_mode(&path, 0o600)?;
    let args = parse_launch(&bootstrap_args(&path))?;
    let startup = startup_plan(&args, "9.9.9")?;
    assert_eq!(
        startup.startup_line,
        "AnyBot 伺服器 v9.9.9：聽 127.0.0.1:8787、[::1]:8787（資料庫 localhost/anybot_test）"
    );
    assert!(!startup.startup_line.contains("fixture-not-a-secret"));
    set_mode(&path, 0o644)
}

#[tokio::test]
async fn t_install_n22_3() -> Result<(), String> {
    let held = TcpListener::bind("127.0.0.1:8787")
        .await
        .map_err(|e| e.to_string())?;
    let failure = match bind_plan(8787).await {
        Ok(_listener) => return Err("埠被占用時不應該成功".to_owned()),
        Err(failure) => failure,
    };
    assert_eq!(failure.code, ExitCode::from(1));
    assert!(failure.message.contains("無法使用連接埠 8787"));
    drop(held);
    Ok(())
}

#[tokio::test]
async fn t_install_n22_4() -> Result<(), String> {
    let args = parse_launch(&[])?;
    let failure = match startup_plan(&args, "9.9.9") {
        Ok(_plan) => return Err("缺啟動檔時不應該成功".to_owned()),
        Err(failure) => failure,
    };
    assert_eq!(failure.code, ExitCode::from(2));
    assert!(failure.message.contains("找不到啟動檔"));
    assert!(failure.message.contains("--bootstrap"));
    let unknown = parse_launch(&[
        "--bootstrap".to_owned(),
        "x.conf".to_owned(),
        "--port".to_owned(),
        "1".to_owned(),
    ]);
    match unknown {
        Ok(_args) => return Err("未知參數不應該成功".to_owned()),
        Err(failure) => assert_eq!(failure.code, ExitCode::from(2)),
    }
    Ok(())
}

#[tokio::test]
async fn t_install_n22_2() -> Result<(), String> {
    let help = anybot_app_server::help_text();
    assert!(help.contains("reset-password"));
    assert!(help.contains("logs"));
    assert!(help.contains("update-bot-computers"));
    assert!(help.contains("migrate-old"));
    assert!(help.contains("--bootstrap"));
    assert!(help.contains("--version"));
    Ok(())
}

#[tokio::test]
async fn t_server_n14_2() -> Result<(), String> {
    let path = fixture("world-readable.conf");
    set_mode(&path, 0o644)?;
    let args = parse_launch(&bootstrap_args(&path))?;
    let failure = match startup_plan(&args, "9.9.9") {
        Ok(_plan) => return Err("權限太開放時不應該成功".to_owned()),
        Err(failure) => failure,
    };
    assert_ne!(failure.code, ExitCode::SUCCESS);
    assert!(failure.message.contains("啟動檔的權限太開放"));
    Ok(())
}
