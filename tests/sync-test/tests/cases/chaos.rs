use std::sync::Arc;
use std::time::Duration;

use anybot_foundation::{Clock, Secret, TrustedCerts};
use anybot_sync::http_wire::HttpWire;
use anybot_sync::{NewRecord, StreamMark, SyncClient, Wire};

use test_harness::timeout_real;

use crate::common::{CLIENT_VERSION, Counting, Rig, drive_until, message, rig};
use crate::scene::chaos::Chaos;
use crate::scene::{
    BOT_STREAM_ID, bot, create_bot, local_bot_records, phone_sends, same_as_server,
};

const STEP: Duration = Duration::from_millis(250);
const SETTLE_STEP: Duration = Duration::from_secs(2);
const TOTAL_RECORDS: u64 = 10_000;
const DEVICES: usize = 3;

struct Device {
    client: SyncClient,
    chaos: Arc<Chaos>,
}

async fn start_device(rig: &Rig, number: usize, seed: u64) -> Result<Device, String> {
    let dir = test_harness::test_dir(&format!("sync_after_2_device_{number}"))?;
    let clock: Arc<dyn Clock> = rig.clock_dyn();
    let http = HttpWire::new(
        &rig.url(),
        &Secret::from_untrusted(rig.bearer.clone()),
        CLIENT_VERSION,
        &TrustedCerts::empty(),
        None,
        Arc::clone(&clock),
    )?;
    let chaos = Chaos::new(Arc::new(http), seed);
    let wire: Arc<dyn Wire> = Arc::<Chaos>::clone(&chaos);
    let random = Counting::arc(number as u64);
    let client = SyncClient::start_with_wire(&dir, wire, clock, random)
        .await
        .map_err(|e| e.to_string())?;
    Ok(Device { client, chaos })
}

async fn device_writes(device: &Device, count: u64, tag: usize, round: u64) -> Result<(), String> {
    for number in 0..count {
        let record = NewRecord {
            stream: bot(),
            kind: "message".to_owned(),
            content: message(&format!("d{tag}-{round}-{number}")),
            task_id: None,
        };
        device
            .client
            .add_record(record)
            .await
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

async fn news(devices: &[Device], rig: &Rig) -> Result<(), String> {
    let rows = rig.server_rows(BOT_STREAM_ID).await?;
    let Some(last) = rows.last() else {
        return Ok(());
    };
    let mark = StreamMark {
        stream: BOT_STREAM_ID.to_owned(),
        seq: last.seq,
        record_id: last.device_id.clone(),
    };
    for device in devices {
        device
            .client
            .hooks()
            .server_has_news(&mark)
            .await
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

async fn all_equal(devices: &[Device], rig: &Rig) -> Result<bool, String> {
    let rows = rig.server_rows(BOT_STREAM_ID).await?;
    for device in devices {
        let local = local_bot_records(&device.client).await?;
        if !same_as_server(&local, &rows) {
            return Ok(false);
        }
    }
    Ok(true)
}

async fn workload(rig: &Rig, devices: &[Device]) -> Result<(), String> {
    let mut written: u64 = 0;
    let mut round: u64 = 0;
    while written < TOTAL_RECORDS {
        let left = TOTAL_RECORDS.saturating_sub(written);
        let phone = (round.saturating_mul(7).checked_rem(40).unwrap_or(0))
            .saturating_add(5)
            .min(left);
        phone_sends(
            rig,
            10_000u64.saturating_add(written),
            phone,
            &format!("r{round}"),
        )
        .await?;
        written = written.saturating_add(phone);
        for (index, device) in devices.iter().enumerate() {
            let left = TOTAL_RECORDS.saturating_sub(written);
            let own = (round
                .saturating_add(index as u64)
                .checked_rem(9)
                .unwrap_or(0))
            .saturating_add(1)
            .min(left);
            device_writes(device, own, index, round).await?;
            written = written.saturating_add(own);
        }
        news(devices, rig).await?;
        let _woken = rig.clock().advance(STEP);
        round = round.saturating_add(1);
    }
    Ok(())
}

#[tokio::test]
async fn t_sync_after_2() -> Result<(), String> {
    timeout_real(after_two(), Duration::from_secs(60)).await?
}

async fn after_two() -> Result<(), String> {
    let rig = rig("sync_after_2").await?;
    create_bot(&rig).await?;
    let mut devices = Vec::new();
    for number in 0..DEVICES {
        devices
            .push(start_device(&rig, number, 0x5eed_0000_u64.saturating_add(number as u64)).await?);
    }
    workload(&rig, &devices).await?;
    let mut rounds = 0u32;
    drive_until(
        "all three replicas equal the server",
        &rig.clock(),
        SETTLE_STEP,
        || {
            rounds = rounds.saturating_add(1);
            let step = rounds;
            let devices = &devices;
            let rig = &rig;
            async move {
                if step.checked_rem(5) == Some(0) {
                    news(devices, rig).await?;
                }
                all_equal(devices, rig).await
            }
        },
    )
    .await?;
    let rows = rig.server_rows(BOT_STREAM_ID).await?;
    assert_eq!(rows.len() as u64, TOTAL_RECORDS);
    let events: u64 = devices.iter().map(|d| d.chaos.tally().events()).sum();
    assert!(events >= 10_000, "only {events} disturbed deliveries");
    for device in &devices {
        assert_eq!(device.client.status().pending, 0);
    }
    drop(devices);
    rig.served.stop().await
}
