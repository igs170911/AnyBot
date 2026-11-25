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

async fn watch_target(watched: Arc<NetCtl>, port: u16, initial: TcpListener) {
    let _initial = initial;
    loop {
        let spec = match watched.target(port).await {
            Ok(spec) => spec,
            Err(_gone) => return,
        };
        if !spec.open {
            close_listener(&watched, port).await;
            tokio::time::sleep(Duration::from_millis(20)).await;
            continue;
        }
        let rebind = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port)).await;
        let Ok(proxy_listener) = rebind else {
            return;
        };
        let wake = watched.wake_for(port).await;
        accept_loop(&watched, port, proxy_listener, wake).await;
    }
}

async fn close_listener(watched: &NetCtl, port: u16) {
    let mut state = watched.state.write().await;
    let Some(entry) = state.entries.get_mut(&port) else {
        return;
    };
    entry.live.clear();
    let waker = Arc::clone(&entry.wake);
    waker.notify_waiters();
}

async fn accept_loop(watched: &NetCtl, port: u16, proxy_listener: TcpListener, wake: Arc<Notify>) {
    loop {
        let accept = tokio::select! {
            () = wake.notified() => None,
            pair = proxy_listener.accept() => Some(pair),
        };
        let Some(Ok((stream, _peer))) = accept else {
            return;
        };
        let kill = Arc::new(Notify::new());
        {
            let mut state = watched.state.write().await;
            let Some(entry) = state.entries.get_mut(&port) else {
                return;
            };
            if !entry.open {
                return;
            }
            entry.live.push(Arc::clone(&kill));
        }
        watched.register_kill(Arc::clone(&kill)).await;
        let Ok(spec) = watched.target(port).await else {
            return;
        };
        netproxy::serve_target(stream, spec, kill).await;
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
    ctl.register(
        proxy_port,
        SocketAddr::from((std::net::Ipv4Addr::LOCALHOST, 0)),
        Some(source),
    )
    .await
}
