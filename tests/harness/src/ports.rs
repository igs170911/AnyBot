use std::net::{Ipv6Addr, TcpListener as StdTcpListener};

pub fn bind_v4() -> Result<(StdTcpListener, u16), String> {
    let listener = StdTcpListener::bind(("127.0.0.1", 0))
        .map_err(|e| format!("failed to bind an ephemeral IPv4 port: {e}"))?;
    let port = listener
        .local_addr()
        .map_err(|e| format!("failed to read the IPv4 port: {e}"))?
        .port();
    Ok((listener, port))
}

pub fn bind_v6() -> Result<(StdTcpListener, u16), String> {
    let listener = StdTcpListener::bind((Ipv6Addr::LOCALHOST, 0))
        .map_err(|e| format!("failed to bind an ephemeral IPv6 port: {e}"))?;
    let port = listener
        .local_addr()
        .map_err(|e| format!("failed to read the IPv6 port: {e}"))?
        .port();
    Ok((listener, port))
}

pub fn bind_pair() -> Result<(StdTcpListener, StdTcpListener, u16), String> {
    let (v4, port) = bind_v4()?;
    let v6 = StdTcpListener::bind((Ipv6Addr::LOCALHOST, port))
        .map_err(|e| format!("failed to bind IPv6 port {port}: {e}"))?;
    Ok((v4, v6, port))
}

pub fn free_dual_port() -> Result<u16, String> {
    let (v4, v6, port) = bind_pair()?;
    drop(v4);
    drop(v6);
    Ok(port)
}

#[must_use]
pub fn port_is_free(port: u16) -> bool {
    StdTcpListener::bind(("127.0.0.1", port)).is_ok()
        && StdTcpListener::bind((Ipv6Addr::LOCALHOST, port)).is_ok()
}
