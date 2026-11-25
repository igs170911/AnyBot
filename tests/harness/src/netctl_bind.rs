use std::net::SocketAddr;

use tokio::net::TcpListener;

pub fn std_bind(port: u16) -> Result<std::net::TcpListener, std::io::Error> {
    let addr = SocketAddr::from((std::net::Ipv4Addr::LOCALHOST, port));
    let sock = socket2::Socket::new(
        socket2::Domain::IPV4,
        socket2::Type::STREAM,
        Some(socket2::Protocol::TCP),
    )?;
    sock.set_reuse_address(true)?;
    sock.bind(&socket2::SockAddr::from(addr))?;
    sock.listen(128)?;
    sock.set_nonblocking(true)?;
    let std_listener: std::net::TcpListener = sock.into();
    Ok(std_listener)
}

pub fn std_bind_any() -> Result<std::net::TcpListener, std::io::Error> {
    let addr = SocketAddr::from(([0, 0, 0, 0], 0));
    std_bind(addr.port())
}

pub async fn tokio_bind(port: u16) -> Result<TcpListener, String> {
    let spawned = tokio::task::spawn_blocking(move || std_bind(port));
    let joined = spawned
        .await
        .map_err(|e| format!("bind join failed: {e}"))?;
    let std_listener = joined.map_err(|e| format!("bind failed: {e}"))?;
    TcpListener::from_std(std_listener).map_err(|e| format!("tokio bind: {e}"))
}
