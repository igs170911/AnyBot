use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt, BufStream};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Notify;

use crate::netctl::NetMode;
use crate::netproxy_flow::{copy_bidirectional_with_kill, sleep_with_kill};

#[derive(Clone)]
pub struct TargetSpec {
    pub real: SocketAddr,
    pub mode: NetMode,
    pub source: Option<IpAddr>,
    pub alias: Option<String>,
    pub open: bool,
    pub wake: Arc<Notify>,
}

pub async fn serve_target(client: TcpStream, spec: TargetSpec, kill: Arc<Notify>) {
    let delay = connect_delay(&spec.mode);
    if !delay.is_zero() {
        sleep_with_kill(delay, &kill).await;
    }
    let mut buffered = BufStream::new(client);
    let head = read_head(&mut buffered).await;
    let Ok(head) = head else {
        return;
    };
    if head.starts_with("CONNECT") {
        let Ok(authority) = parse_connect_authority(&head) else {
            return;
        };
        let target = aliased_connect(&spec, authority);
        let Ok(mut upstream) = connect_real_addr(&spec, target).await else {
            return;
        };
        let Ok(_written) = buffered
            .write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")
            .await
        else {
            return;
        };
        let Ok(_flushed) = buffered.flush().await else {
            return;
        };
        let mut client_stream = buffered.into_inner();
        let _copied =
            copy_bidirectional_with_kill(&mut client_stream, &mut upstream, &spec.mode, &kill)
                .await
                .is_ok();
        return;
    }
    let Ok(mut upstream) = connect_real(&spec).await else {
        return;
    };
    let rewritten = rewrite_host(&head, spec.alias.as_deref(), spec.real);
    let Ok(_sent) = upstream.write_all(rewritten.as_bytes()).await else {
        return;
    };
    let Ok(_flushed) = upstream.flush().await else {
        return;
    };
    let mut client_stream = buffered.into_inner();
    let _copied =
        copy_bidirectional_with_kill(&mut client_stream, &mut upstream, &spec.mode, &kill)
            .await
            .is_ok();
}

fn connect_delay(mode: &NetMode) -> Duration {
    match mode {
        NetMode::Slow { connect, .. } => *connect,
        NetMode::Open
        | NetMode::Cut
        | NetMode::Buffer
        | NetMode::IdleCut { .. }
        | NetMode::Down
        | NetMode::HangConnect => Duration::ZERO,
    }
}

async fn read_head(buffered: &mut BufStream<TcpStream>) -> Result<String, String> {
    let mut head = Vec::new();
    let mut byte = [0u8; 1];
    loop {
        let read = buffered.read(&mut byte).await;
        let Ok(1) = read else {
            return Err("read head failed".to_owned());
        };
        head.push(byte[0]);
        if head.ends_with(b"\r\n\r\n") {
            let Ok(text) = String::from_utf8(head) else {
                return Err("head not utf8".to_owned());
            };
            return Ok(text);
        }
        if head.len() > 16 * 1024 {
            return Err("head too large".to_owned());
        }
    }
}

fn aliased_connect(spec: &TargetSpec, authority: SocketAddr) -> SocketAddr {
    let Some(name) = spec.alias.as_deref() else {
        return authority;
    };
    let host = authority.ip().to_string();
    if host == name || authority.to_string().starts_with(name) {
        return spec.real;
    }
    authority
}

#[must_use]
pub fn rewrite_host(head: &str, alias: Option<&str>, real: SocketAddr) -> String {
    let Some(name) = alias else {
        return head.to_owned();
    };
    let needle = format!("Host: {name}");
    let replacement = format!("Host: {real}");
    head.replace(&needle, &replacement)
}

fn parse_connect_authority(head: &str) -> Result<SocketAddr, String> {
    let Some(first_line) = head.lines().next() else {
        return Err("no first line".to_owned());
    };
    let parts: Vec<&str> = first_line.split_whitespace().collect();
    if parts.len() < 2 || parts.first().is_none_or(|p| *p != "CONNECT") {
        return Err("not a CONNECT line".to_owned());
    }
    let Some(authority) = parts.get(1) else {
        return Err("CONNECT line missing authority".to_owned());
    };
    let Ok(addr) = authority.parse::<SocketAddr>() else {
        let Some((host, port)) = authority.split_once(':') else {
            return Err("authority missing port".to_owned());
        };
        let Ok(port) = port.parse::<u16>() else {
            return Err("authority port not a number".to_owned());
        };
        let Ok(ip) = host.parse::<IpAddr>() else {
            return Err("authority host not an IP".to_owned());
        };
        return Ok(SocketAddr::from((ip, port)));
    };
    Ok(addr)
}

async fn connect_real(spec: &TargetSpec) -> Result<TcpStream, String> {
    connect_real_addr(spec, spec.real).await
}

async fn connect_real_addr(spec: &TargetSpec, addr: SocketAddr) -> Result<TcpStream, String> {
    if matches!(spec.mode, NetMode::Cut | NetMode::Down) {
        return Err("network cut".to_owned());
    }
    if matches!(spec.mode, NetMode::HangConnect) {
        std::future::pending::<()>().await;
        return Err("connect hung".to_owned());
    }
    let Some(source) = spec.source else {
        return TcpStream::connect(addr)
            .await
            .map_err(|e| format!("connect to {addr} failed: {e}"));
    };
    bind_connect(source, addr).await
}

fn sockaddr_ip(ip: IpAddr) -> socket2::SockAddr {
    socket2::SockAddr::from(SocketAddr::from((ip, 0)))
}

async fn bind_connect(source: IpAddr, addr: SocketAddr) -> Result<TcpStream, String> {
    let domain = if addr.is_ipv4() {
        socket2::Domain::IPV4
    } else {
        socket2::Domain::IPV6
    };
    let Ok(socket) =
        socket2::Socket::new(domain, socket2::Type::STREAM, Some(socket2::Protocol::TCP))
    else {
        return Err("socket creation failed".to_owned());
    };
    if socket.bind(&sockaddr_ip(source)).is_err() {
        return Err(format!("bind source {source} failed"));
    }
    let std_stream: std::net::TcpStream = socket.into();
    let tokio_socket = tokio::net::TcpSocket::from_std_stream(std_stream);
    tokio_socket
        .connect(addr)
        .await
        .map_err(|e| format!("connect to {addr} failed: {e}"))
}

pub async fn listen_open(port: u16) -> Result<TcpListener, String> {
    TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port))
        .await
        .map_err(|e| format!("bind {port} failed: {e}"))
}
