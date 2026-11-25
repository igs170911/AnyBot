use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

use tokio::net::TcpListener;
use tokio::sync::Notify;

use crate::netctl_state::NetCtl;
use crate::netproxy;
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum NetMode {
    Open,
    Cut,
    Slow {
        connect: Duration,
        first_byte: Duration,
        byte_gap: Duration,
    },
    Buffer,
    IdleCut {
        idle: Duration,
    },
    Down,
}

async fn wait_open(watched: &Arc<NetCtl>, port: u16) -> bool {
    loop {
        let Ok(spec) = watched.target(port).await else {
            return false;
        };
        if spec.open {
            return true;
        }
        close_listener(watched, port).await;
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

async fn watch_target(watched: Arc<NetCtl>, port: u16, initial: TcpListener) {
    accept_loop_for(&watched, port, initial).await;
    loop {
        if !wait_open(&watched, port).await {
            return;
        }
        let Ok(proxy_listener) = rebind_with_retry(&watched, port).await else {
            return;
        };
        accept_loop_for(&watched, port, proxy_listener).await;
    }
}

pub async fn net_start() -> Result<(Arc<NetCtl>, u16), String> {
    let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
        .await
        .map_err(|e| format!("bind proxy failed: {e}"))?;
    let addr = listener
        .local_addr()
        .map_err(|e| format!("proxy local addr failed: {e}"))?;
    let ctl = Arc::new(NetCtl::new());
    let port = addr.port();
    ctl.open_target(port, addr).await;
    let watched = Arc::clone(&ctl);
    tokio::spawn(async move { watch_target(watched, port, listener).await });
    Ok((ctl, port))
}

async fn rebind_with_retry(watched: &Arc<NetCtl>, port: u16) -> Result<TcpListener, ()> {
    let Ok(spec) = watched.target(port).await else {
        return Err(());
    };
    if !spec.open {
        close_listener(watched, port).await;
        return Err(());
    }
    bind_loop(port).await
}

fn std_bind(port: u16) -> Result<std::net::TcpListener, std::io::Error> {
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

async fn bind_loop(port: u16) -> Result<TcpListener, ()> {
    for _round in 0..40 {
        let spawn = tokio::task::spawn_blocking(move || std_bind(port));
        let joined = tokio::time::timeout(Duration::from_secs(5), spawn).await;
        let Ok(Ok(Ok(std_listener))) = joined else {
            tracing::debug!("proxy rebind busy port {port}");
            tokio::time::sleep(Duration::from_millis(500)).await;
            continue;
        };
        let Ok(listener) = TcpListener::from_std(std_listener) else {
            tokio::time::sleep(Duration::from_millis(500)).await;
            continue;
        };
        tracing::debug!("proxy rebound port {port}");
        return Ok(listener);
    }
    Err(())
}

async fn bind_attempt(port: u16) -> Option<TcpListener> {
    TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port))
        .await
        .ok()
}

async fn slot_open(watched: &NetCtl, port: u16) -> Option<bool> {
    let state = watched.state.read().await;
    state.entries.get(&port).map(|entry| entry.open)
}

async fn register_slot(watched: &NetCtl, port: u16) -> Option<Arc<Notify>> {
    let open = slot_open(watched, port).await;
    let Some(true) = open else {
        tracing::debug!("raw loop unavailable port {port} state {open:?}");
        return None;
    };
    Some(Arc::new(Notify::new()))
}

fn slot_after(closed: bool) -> Slot {
    if closed { Slot::Stop } else { Slot::Idle }
}

enum Slot {
    Stop,
    Idle,
    Connect(tokio::net::TcpStream),
}

async fn accept_ready(
    accept: Option<Result<(tokio::net::TcpStream, SocketAddr), std::io::Error>>,
    watched: &NetCtl,
    port: u16,
) -> Slot {
    let closed = watched.is_closed(port).await;
    match (accept, closed) {
        (Some(Ok((stream, _peer))), _) => Slot::Connect(stream),
        (Some(Err(err)), _) => {
            tracing::debug!("proxy accept error {err} port {port}");
            slot_after(closed)
        }
        (None, _) => slot_after(closed),
    }
}

async fn spawn_service(
    watched: &NetCtl,
    port: u16,
    stream: tokio::net::TcpStream,
    kill: Arc<Notify>,
) -> bool {
    tracing::debug!("proxy accepted port {port}");
    watched.register_kill(Arc::clone(&kill)).await;
    let Ok(spec) = watched.target(port).await else {
        return false;
    };
    netproxy::serve_target(stream, spec, kill).await;
    true
}

async fn spawn_raw_service(
    watched: &NetCtl,
    port: u16,
    stream: tokio::net::TcpStream,
    kill: Arc<Notify>,
) -> bool {
    tracing::debug!("raw accepted port {port}");
    watched.register_kill(Arc::clone(&kill)).await;
    let Ok(spec) = watched.target(port).await else {
        return false;
    };
    tokio::spawn(crate::netproxy_raw::serve_raw_target(stream, spec, kill));
    true
}

async fn accept_loop_for(watched: &Arc<NetCtl>, port: u16, proxy_listener: TcpListener) {
    let wake = watched.wake_for(port).await;
    accept_loop(watched, port, proxy_listener, wake).await;
}

async fn close_listener(watched: &NetCtl, port: u16) {
    let waker = {
        let mut state = watched.state.write().await;
        let Some(entry) = state.entries.get_mut(&port) else {
            return;
        };
        entry.live.clear();
        Arc::clone(&entry.wake)
    };
    waker.notify_waiters();
}

async fn accept_loop(watched: &NetCtl, port: u16, proxy_listener: TcpListener, wake: Arc<Notify>) {
    loop {
        let Some(kill) = register_slot(watched, port).await else {
            return;
        };
        let accept = tokio::select! {
            () = wake.notified() => None,
            pair = proxy_listener.accept() => Some(pair),
            () = tokio::time::sleep(Duration::from_millis(100)) => None,
        };
        match accept_ready(accept, watched, port).await {
            Slot::Stop => return,
            Slot::Idle => {}
            Slot::Connect(stream) => {
                if !spawn_service(watched, port, stream, kill).await {
                    return;
                }
            }
        }
    }
}

pub async fn net_start_raw() -> Result<(Arc<NetCtl>, u16), String> {
    let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
        .await
        .map_err(|e| format!("bind raw proxy failed: {e}"))?;
    let addr = listener
        .local_addr()
        .map_err(|e| format!("raw proxy local addr failed: {e}"))?;
    let ctl = Arc::new(NetCtl::new());
    let port = addr.port();
    ctl.open_target(port, addr).await;
    let watched = Arc::clone(&ctl);
    tokio::spawn(async move { watch_raw_target(watched, port, listener).await });
    Ok((ctl, port))
}

async fn watch_raw_target(watched: Arc<NetCtl>, port: u16, initial: TcpListener) {
    raw_accept_loop_for(&watched, port, initial).await;
    loop {
        if !wait_open(&watched, port).await {
            return;
        }
        let Ok(proxy_listener) = rebind_with_retry(&watched, port).await else {
            return;
        };
        raw_accept_loop_for(&watched, port, proxy_listener).await;
    }
}

async fn raw_accept_loop_for(watched: &Arc<NetCtl>, port: u16, proxy_listener: TcpListener) {
    let wake = watched.wake_for(port).await;
    let _looped = raw_accept_loop(watched, port, proxy_listener, wake).await;
}

async fn raw_accept_loop(
    watched: &NetCtl,
    port: u16,
    proxy_listener: TcpListener,
    wake: Arc<Notify>,
) {
    loop {
        let Some(kill) = register_slot(watched, port).await else {
            return;
        };
        let accept = tokio::select! {
            () = wake.notified() => None,
            pair = proxy_listener.accept() => Some(pair),
            () = tokio::time::sleep(Duration::from_millis(100)) => None,
        };
        match accept_ready(accept, watched, port).await {
            Slot::Stop => return,
            Slot::Idle => {}
            Slot::Connect(stream) => {
                if !spawn_raw_service(watched, port, stream, kill).await {
                    return;
                }
            }
        }
    }
}

pub async fn net_attach(
    ctl: &Arc<NetCtl>,
    proxy_port: u16,
    real: SocketAddr,
) -> Result<(), String> {
    ctl.register(proxy_port, real, None).await
}

pub async fn net_mode(ctl: &Arc<NetCtl>, proxy_port: u16) -> NetMode {
    current_mode(ctl, proxy_port).await
}

pub async fn current_mode(ctl: &Arc<NetCtl>, proxy_port: u16) -> NetMode {
    let state = ctl.state.read().await;
    state
        .entries
        .get(&proxy_port)
        .map_or(NetMode::Open, |entry| {
            if state.global_cut {
                NetMode::Cut
            } else {
                entry.mode
            }
        })
}

pub async fn net_cut(ctl: &Arc<NetCtl>, proxy_port: u16) -> Result<(), String> {
    ctl.set_mode(proxy_port, NetMode::Cut).await?;
    ctl.kill_live().await;
    Ok(())
}

pub async fn net_probe_open(ctl: &Arc<NetCtl>, proxy_port: u16) -> Result<bool, String> {
    let spec = ctl.target(proxy_port).await?;
    Ok(spec.open)
}

pub async fn net_restore(ctl: &Arc<NetCtl>, proxy_port: u16) -> Result<(), String> {
    ctl.set_mode(proxy_port, NetMode::Open).await
}

pub async fn net_cut_target(ctl: &Arc<NetCtl>, _proxy_port: u16) -> Result<(), String> {
    ctl.cut_all().await
}

pub async fn net_restore_target(ctl: &Arc<NetCtl>, _proxy_port: u16) -> Result<(), String> {
    ctl.restore_all().await
}

pub async fn net_slow(
    ctl: &Arc<NetCtl>,
    proxy_port: u16,
    connect_delay: Duration,
) -> Result<(), String> {
    ctl.set_mode(
        proxy_port,
        NetMode::Slow {
            connect: connect_delay,
            first_byte: connect_delay,
            byte_gap: connect_delay,
        },
    )
    .await?;
    ctl.kill_live().await;
    Ok(())
}

pub async fn net_buffer(ctl: &Arc<NetCtl>, proxy_port: u16, on: bool) -> Result<(), String> {
    let mode = if on {
        NetMode::Buffer
    } else {
        current_mode(ctl, proxy_port).await
    };
    ctl.set_mode(proxy_port, mode).await?;
    ctl.kill_live().await;
    Ok(())
}

pub async fn net_idle_cut(
    ctl: &Arc<NetCtl>,
    proxy_port: u16,
    idle: Duration,
) -> Result<(), String> {
    ctl.set_mode(proxy_port, NetMode::IdleCut { idle }).await?;
    ctl.kill_live().await;
    Ok(())
}

pub async fn net_connect_tunnel(ctl: &Arc<NetCtl>, proxy_port: u16) -> Result<(), String> {
    ctl.set_mode(proxy_port, NetMode::Open).await?;
    ctl.kill_live().await;
    Ok(())
}

pub async fn net_source_addr(
    ctl: &Arc<NetCtl>,
    proxy_port: u16,
    source: IpAddr,
) -> Result<(), String> {
    ctl.set_source(proxy_port, Some(source)).await
}

pub async fn net_down(ctl: &Arc<NetCtl>, proxy_port: u16) -> Result<(), String> {
    ctl.set_mode(proxy_port, NetMode::Down).await?;
    ctl.kill_live().await;
    Ok(())
}
