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
    let out = vec![bind_one(v4)?, bind_one(v6)?];
    Ok(out)
}

fn bind_one(addr: SocketAddr) -> std::io::Result<TcpListener> {
    let domain = if addr.is_ipv4() {
        socket2::Domain::IPV4
    } else {
        socket2::Domain::IPV6
    };
    let socket = socket2::Socket::new(domain, socket2::Type::STREAM, None)?;
    socket.set_reuse_address(true)?;
    socket.set_reuse_port(true)?;
    socket.bind(&addr.into())?;
    socket.listen(128)?;
    let std_listener: std::net::TcpListener = socket.into();
    std_listener.set_nonblocking(true)?;
    TcpListener::from_std(std_listener)
}
