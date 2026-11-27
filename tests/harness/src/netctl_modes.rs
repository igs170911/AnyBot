use std::sync::Arc;

use crate::fake_clock::FakeClock;
use crate::netctl::NetMode;
use crate::netctl_state::NetCtl;

pub async fn net_throttle(
    ctl: &Arc<NetCtl>,
    proxy_port: u16,
    bytes_per_sec: u32,
    clock: &Arc<FakeClock>,
) -> Result<(), String> {
    ctl.set_clock(proxy_port, Some(Arc::clone(clock))).await;
    ctl.set_mode(proxy_port, NetMode::Throttle { bytes_per_sec })
        .await?;
    ctl.kill_live().await;
    Ok(())
}

pub async fn net_drop_events(
    ctl: &Arc<NetCtl>,
    proxy_port: u16,
    per_mille: u16,
    seed: u64,
) -> Result<(), String> {
    ctl.set_mode(proxy_port, NetMode::DropEvents { per_mille, seed })
        .await?;
    ctl.with_entry(proxy_port, |entry| {
        entry
            .event_rng
            .store(seed, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    })
    .await?;
    ctl.kill_live().await;
    Ok(())
}
