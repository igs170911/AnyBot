use std::net::SocketAddr;

use tokio::net::TcpListener;

use anybot_foundation::{FoundationError, ServerBootstrap, ServerLaunchArgs};

pub const DEFAULT_PORT: u16 = 8787;

#[derive(Debug)]
pub struct ServerStartup {
    pub startup_line: String,
    pub bootstrap: ServerBootstrap,
}

#[must_use]
pub fn startup_line(version: &str, port: u16, bootstrap: &ServerBootstrap) -> String {
    format!(
        "AnyBot 伺服器 v{version}：聽 127.0.0.1:{port}、[::1]:{port}（資料庫 {}）",
        bootstrap.describe_db()
    )
}

pub fn prepare(args: &ServerLaunchArgs, version: &str) -> Result<ServerStartup, FoundationError> {
    let path = args
        .bootstrap
        .as_ref()
        .ok_or(FoundationError::BootstrapNotFound(
            "找不到啟動檔（用 --bootstrap 指定）".to_owned(),
        ))?;
    let bootstrap = ServerBootstrap::read(path)?;
    Ok(ServerStartup {
        startup_line: startup_line(version, DEFAULT_PORT, &bootstrap),
        bootstrap,
    })
}

pub async fn bind_listeners(port: u16) -> std::io::Result<Vec<TcpListener>> {
    let v4 = SocketAddr::from(([127, 0, 0, 1], port));
    let v6 = SocketAddr::from((std::net::Ipv6Addr::LOCALHOST, port));
    let mut out = Vec::new();
    out.push(TcpListener::bind(v4).await?);
    out.push(TcpListener::bind(v6).await?);
    Ok(out)
}
