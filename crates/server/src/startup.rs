use std::net::SocketAddr;

use tokio::net::TcpListener;

use anybot_foundation::{
    CODE_BOOTSTRAP_MISSING, FoundationError, MessageRef, ServerBootstrap, ServerLaunchArgs,
};

pub const DEFAULT_PORT: u16 = 8787;
pub use anybot_foundation::CODE_SERVER_STARTUP_LISTENING;

pub const CODE_SERVER_CONFIG_INVALID: &str = "server-config-invalid";

#[derive(Debug)]
pub struct ServerStartup {
    pub startup: MessageRef,
    pub bootstrap: ServerBootstrap,
}

#[must_use]
pub fn startup_message(
    version: &str,
    addresses: &[String],
    port: u16,
    bootstrap: &ServerBootstrap,
) -> MessageRef {
    let listed: Vec<String> = addresses.iter().map(|a| bracketed(a, port)).collect();
    MessageRef::new(CODE_SERVER_STARTUP_LISTENING)
        .with_text("version", version)
        .with_text("addresses", &listed.join(", "))
        .with_text("db_host", &bootstrap.host)
        .with_text("db_name", &bootstrap.database)
}

fn bracketed(address: &str, port: u16) -> String {
    if address.contains(':') {
        return format!("[{address}]:{port}");
    }
    format!("{address}:{port}")
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
        startup: startup_message(
            version,
            &["127.0.0.1".to_owned(), "::1".to_owned()],
            DEFAULT_PORT,
            &bootstrap,
        ),
        bootstrap,
    })
}

pub async fn bind_listeners(port: u16) -> std::io::Result<Vec<TcpListener>> {
    let v4 = SocketAddr::from(([127, 0, 0, 1], port));
    let v6 = SocketAddr::from((std::net::Ipv6Addr::LOCALHOST, port));
    let out = vec![bind_one(v4)?, bind_one(v6)?];
    Ok(out)
}

pub fn parse_listen_host(text: &str, port: u16) -> Result<SocketAddr, String> {
    let trimmed = text.trim();
    if trimmed.contains(':') && trimmed.matches(':').count() == 1 && !trimmed.contains('[') {
        let Ok(addr) = trimmed.parse::<SocketAddr>() else {
            return Err("not an address with a port".to_owned());
        };
        return Ok(SocketAddr::new(addr.ip(), port));
    }
    if trimmed.contains('[') {
        let Ok(addr) = trimmed.parse::<SocketAddr>() else {
            return Err("not an address with a port".to_owned());
        };
        return Ok(SocketAddr::new(addr.ip(), port));
    }
    let Ok(ip) = trimmed.parse::<std::net::IpAddr>() else {
        return Err("not an IP address".to_owned());
    };
    Ok(SocketAddr::from((ip, port)))
}

pub async fn bind_addrs(addresses: &[SocketAddr]) -> std::io::Result<Vec<TcpListener>> {
    let mut out = Vec::new();
    for addr in addresses {
        out.push(bind_one(*addr)?);
    }
    Ok(out)
}

fn bind_one(addr: SocketAddr) -> std::io::Result<TcpListener> {
    let domain = if addr.is_ipv4() {
        socket2::Domain::IPV4
    } else {
        socket2::Domain::IPV6
    };
    let socket = socket2::Socket::new(domain, socket2::Type::STREAM, None)?;
    if addr.is_ipv6() {
        socket.set_only_v6(true)?;
    }
    socket.set_reuse_address(true)?;
    socket.bind(&addr.into())?;
    socket.listen(128)?;
    let std_listener: std::net::TcpListener = socket.into();
    std_listener.set_nonblocking(true)?;
    TcpListener::from_std(std_listener)
}
