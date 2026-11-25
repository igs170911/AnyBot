use std::sync::Arc;

use tokio::io::AsyncWriteExt as _;
use tokio::net::TcpStream;
use tokio::sync::Notify;

use crate::netctl::NetMode;
use crate::netproxy::TargetSpec;
use crate::netproxy::connect_real;
use crate::netproxy_flow::{copy_bidirectional_with_kill, sleep_with_kill};

pub async fn serve_raw_target(client: TcpStream, spec: TargetSpec, kill: Arc<Notify>) {
    tracing::debug!("raw serve start");
    if !spec.mode.connect_delay().is_zero() {
        sleep_with_kill(spec.mode.connect_delay(), &kill).await;
    }
    let Ok(mut upstream) = connect_real(&spec).await else {
        return;
    };
    let Ok(_flushed) = upstream.flush().await else {
        return;
    };
    let mut client = client;
    let _copied = copy_bidirectional_with_kill(&mut client, &mut upstream, &spec.mode, &kill).await;
}

impl NetMode {
    #[must_use]
    pub fn connect_delay(&self) -> std::time::Duration {
        match self {
            NetMode::Slow { connect, .. } => *connect,
            NetMode::Open
            | NetMode::Cut
            | NetMode::HangConnect
            | NetMode::Buffer
            | NetMode::IdleCut { .. }
            | NetMode::Down => std::time::Duration::ZERO,
        }
    }
}
