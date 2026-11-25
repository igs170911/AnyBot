use std::net::SocketAddr;

use tokio::net::TcpListener;

use anybot_foundation::{
    CODE_BOOTSTRAP_MISSING, FoundationError, MessageRef, ServerBootstrap, ServerLaunchArgs,
};

pub const DEFAULT_PORT: u16 = 8787;
pub use anybot_foundation::CODE_SERVER_STARTUP_LISTENING;

#[derive(Debug)]
pub struct ServerStartup {
    pub startup: MessageRef,
    pub bootstrap: ServerBootstrap,
}

#[must_use]
pub fn startup_message(version: &str, port: u16, bootstrap: &ServerBootstrap) -> MessageRef {
    MessageRef::new(CODE_SERVER_STARTUP_LISTENING)
        .with_text("version", version)
        .with_int("port", i64::from(port))
        .with_text("db_host", &bootstrap.host)
        .with_text("db_name", &bootstrap.database)
}

pub fn prepare(args: &ServerLaunchArgs, version: &str) -> Result<ServerStartup, FoundationError> {
    let path = args
        .bootstrap
        .as_ref()
        .ok_or(FoundationError::BootstrapNotFound(MessageRef::new(
            CODE_BOOTSTRAP_MISSING,
        )))?;
    let bootstrap = ServerBootstrap::read(path)?;
    Ok(ServerStartup {
        startup: startup_message(version, DEFAULT_PORT, &bootstrap),
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
    socket.bind(&addr.into())?;
    socket.listen(128)?;
    let std_listener: std::net::TcpListener = socket.into();
    std_listener.set_nonblocking(true)?;
    TcpListener::from_std(std_listener)
}
