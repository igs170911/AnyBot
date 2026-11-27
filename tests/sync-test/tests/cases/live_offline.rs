use std::time::Duration;

use anybot_sync::reconnect_status::LinkPhase;
use test_harness::netctl::{net_down, net_restore};
use test_harness::{test_dir, timeout_real};

use anybot_sync::SyncClient;

use crate::common::{Counting, Rig, drive_until, rig};
use crate::scene::live::{drive_to_phase, phase};
use crate::scene::{create_bot, local_has, phone_sends};

const STEP: Duration = Duration::from_millis(100);

async fn offline_while_phone_sends(rig: &Rig, client: &SyncClient) -> Result<u64, String> {
    let clock = rig.clock();
    drive_to_phase(client, &clock, STEP, LinkPhase::Connected).await?;
    net_down(&rig.ctl, rig.proxy_port).await?;
    drive_to_phase(client, &clock, STEP, LinkPhase::Retrying).await?;
    phone_sends(rig, 1, 1, "n4").await?;
    let _woken = clock.advance(Duration::from_secs(5));
    assert!(!local_has(client, 1).await?, "A is offline");
    net_restore(&rig.ctl, rig.proxy_port).await?;
    drive_to_phase(client, &clock, STEP, LinkPhase::Connected).await?;
    client
        .status()
        .link
        .map(|link| link.since.millis())
        .ok_or("no link status".to_owned())
}

#[tokio::test]
async fn t_account_n4_2() -> Result<(), String> {
    timeout_real(account_n4_2(), Duration::from_secs(90)).await?
}

async fn account_n4_2() -> Result<(), String> {
    let rig = rig("sync_n4_2").await?;
    create_bot(&rig).await?;
    let clock = rig.clock();
    let client = rig
        .client(&test_dir("sync_n4_2_a")?, Counting::arc(1))
        .await?;
    let back_at = offline_while_phone_sends(&rig, &client).await?;
    drive_until("A has the message", &clock, STEP, || local_has(&client, 1)).await?;
    let took = clock.now_ms().saturating_sub(back_at);
    assert!(
        took <= 2_000,
        "the message arrived {took} ms after A came back"
    );
    assert_eq!(phase(&client), Some(LinkPhase::Connected));
    drop(client);
    rig.served.stop().await
}
