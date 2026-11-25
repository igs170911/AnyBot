use std::collections::BTreeMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;

use tokio::sync::Mutex;
use tokio::sync::Notify;
use tokio::sync::RwLock;

use crate::netctl::NetMode;
use crate::netproxy::TargetSpec;

pub struct NetCtl {
    pub(crate) state: RwLock<NetState>,
    pub(crate) kills: Mutex<Vec<Arc<Notify>>>,
}

#[derive(Default)]
pub(crate) struct NetState {
    pub(crate) global_cut: bool,
    pub(crate) entries: BTreeMap<u16, TargetEntry>,
}

pub(crate) struct TargetEntry {
    pub(crate) real: SocketAddr,
    pub(crate) mode: NetMode,
    pub(crate) source: Option<IpAddr>,
    pub(crate) open: bool,
    pub(crate) wake: Arc<Notify>,
    pub(crate) live: Vec<Arc<Notify>>,
}

impl NetCtl {
    pub(crate) fn new() -> Self {
        NetCtl {
            state: RwLock::new(NetState {
                global_cut: false,
                entries: BTreeMap::new(),
            }),
            kills: Mutex::new(Vec::new()),
        }
    }

    pub(crate) async fn open_target(&self, proxy_port: u16, real: SocketAddr) {
        let mut state = self.state.write().await;
        state.entries.insert(
            proxy_port,
            TargetEntry {
                real,
                mode: NetMode::Open,
                source: None,
                open: true,
                wake: Arc::new(Notify::new()),
                live: Vec::new(),
            },
        );
    }

    pub(crate) async fn wake_for(&self, proxy_port: u16) -> Arc<Notify> {
        let state = self.state.read().await;
        let Some(entry) = state.entries.get(&proxy_port) else {
            return Arc::new(Notify::new());
        };
        Arc::clone(&entry.wake)
    }

    pub(crate) async fn target(&self, proxy_port: u16) -> Result<TargetSpec, String> {
        let state = self.state.read().await;
        let Some(entry) = state.entries.get(&proxy_port) else {
            return Err(format!("no target registered for proxy port {proxy_port}"));
        };
        Ok(TargetSpec {
            real: entry.real,
            mode: if state.global_cut {
                NetMode::Cut
            } else {
                entry.mode
            },
            source: entry.source,
            open: entry.open,
            wake: Arc::clone(&entry.wake),
        })
    }

    pub(crate) async fn with_entry<T>(
        &self,
        proxy_port: u16,
        f: impl FnOnce(&mut TargetEntry) -> Result<T, String>,
    ) -> Result<T, String> {
        let mut state = self.state.write().await;
        let Some(entry) = state.entries.get_mut(&proxy_port) else {
            return Err(format!("no target registered for proxy port {proxy_port}"));
        };
        f(entry)
    }

    pub async fn register(
        &self,
        proxy_port: u16,
        real: SocketAddr,
        source: Option<IpAddr>,
    ) -> Result<(), String> {
        self.with_entry(proxy_port, |entry| {
            entry.real = real;
            entry.source = source;
            Ok(())
        })
        .await
    }

    pub async fn set_mode(&self, proxy_port: u16, mode: NetMode) -> Result<(), String> {
        self.with_entry(proxy_port, |entry| {
            entry.mode = mode;
            entry.open = matches!(mode, NetMode::Open);
            Ok(())
        })
        .await
    }

    pub(crate) async fn kill_live(&self) {
        let kills = self.kills.lock().await;
        for kill in kills.iter() {
            kill.notify_waiters();
        }
    }

    pub async fn cut_all(&self) -> Result<(), String> {
        {
            let mut state = self.state.write().await;
            state.global_cut = true;
        }
        self.kill_live().await;
        Ok(())
    }

    pub async fn restore_all(&self) -> Result<(), String> {
        {
            let mut state = self.state.write().await;
            state.global_cut = false;
            for entry in state.entries.values_mut() {
                entry.mode = NetMode::Open;
                entry.open = true;
            }
        }
        self.kill_live().await;
        Ok(())
    }

    pub(crate) async fn register_kill(&self, kill: Arc<Notify>) {
        let mut kills = self.kills.lock().await;
        kills.push(kill);
    }

    pub async fn drain_kills(&self) {
        let mut kills = self.kills.lock().await;
        kills.clear();
    }
}
