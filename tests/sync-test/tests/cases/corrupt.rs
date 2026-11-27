use std::future::Future;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anybot_app_test::fsutil::{read_bytes, write_bytes};
use anybot_store_client::{DATABASE_FILE, Store};
use anybot_sync::{NewRecord, StreamTarget, SyncClient};
use test_harness::netctl::{net_down, net_restore};

use crate::common::{Counting, Rig, drive_until, message, rig};
use crate::scene::tap::Tap;
use crate::scene::{
    BOT_STREAM_ID, create_bot, local_bot_records, local_has, phone_sends, same_as_server,
};

const STEP: Duration = Duration::from_millis(250);

#[derive(Clone, Copy)]
enum Damage {
    NotSqlite,
    DamagedPages,
    NewerSchema,
}

async fn open(rig: &Rig, tap: &Tap, dir: &Path) -> Result<SyncClient, String> {
    rig.client_at(&tap.url(), dir, Counting::arc(0)).await
}

async fn wait_for<F, Fut>(rig: &Rig, what: &str, check: F) -> Result<(), String>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<bool, String>>,
{
    drive_until(what, &rig.clock(), STEP, check).await
}

async fn queued_while_offline(rig: &Rig, tap: &Tap, dir: &Path, queue: bool) -> Result<(), String> {
    create_bot(rig).await?;
    phone_sends(rig, 10, 3, "before").await?;
    let client = open(rig, tap, dir).await?;
    wait_for(rig, "first sync", || async { local_has(&client, 3).await }).await?;
    if queue {
        net_down(&rig.ctl, rig.proxy_port).await?;
        let record = NewRecord {
            stream: StreamTarget::Bot(crate::scene::BOT.to_owned()),
            kind: "message".to_owned(),
            content: message("queued"),
            task_id: None,
        };
        client.add_record(record).await.map_err(|e| e.to_string())?;
    }
    client.close().await;
    Ok(())
}

async fn newer_schema(dir: &Path) -> Result<(), String> {
    let opened = Store::open(dir, 1).await.map_err(|e| e.to_string())?;
    sqlx::query(
        "INSERT INTO _sqlx_migrations (version, description, success, checksum, execution_time)
            VALUES (9999, 'future', 1, x'00', 0)",
    )
    .execute(opened.store.pool())
    .await
    .map_err(|e| e.to_string())?;
    opened.store.close().await;
    Ok(())
}

fn rewrite(path: &Path, damage: Damage) -> Result<(), String> {
    let mut bytes = read_bytes(path)?;
    match damage {
        Damage::NotSqlite => bytes = b"definitely not a database ".repeat(400),
        Damage::DamagedPages => {
            let from = bytes.len().saturating_sub(4096).max(4096);
            for byte in bytes.iter_mut().skip(from) {
                *byte = 0xA5;
            }
        }
        Damage::NewerSchema => return Ok(()),
    }
    write_bytes(path, &bytes)
}

async fn break_database(dir: &Path, damage: Damage) -> Result<Vec<u8>, String> {
    let path = dir.join(DATABASE_FILE);
    if matches!(damage, Damage::NewerSchema) {
        newer_schema(dir).await?;
    } else {
        rewrite(&path, damage)?;
    }
    read_bytes(&path)
}

struct Outcome {
    client: SyncClient,
    broken: Vec<u8>,
    moved_to: PathBuf,
}

async fn reopen_after(rig: &Rig, tap: &Tap, dir: &Path, damage: Damage) -> Result<Outcome, String> {
    let broken = break_database(dir, damage).await?;
    net_restore(&rig.ctl, rig.proxy_port).await?;
    let client = open(rig, tap, dir).await?;
    let notice = client.status().replica_notice.ok_or("no replica notice")?;
    assert_eq!(notice.code, "replica-recovered");
    let expected = match damage {
        Damage::NotSqlite => "replica-not-sqlite",
        Damage::DamagedPages => "replica-damaged",
        Damage::NewerSchema => "replica-newer-schema",
    };
    assert_eq!(notice.reason_code, expected);
    Ok(Outcome {
        client,
        broken,
        moved_to: PathBuf::from(notice.moved_to),
    })
}

async fn assert_resynced(rig: &Rig, outcome: &Outcome, total: usize) -> Result<(), String> {
    let moved = read_bytes(&outcome.moved_to)?;
    assert_eq!(
        moved, outcome.broken,
        "the broken file must be kept byte for byte"
    );
    wait_for(rig, "replica rebuilt from the server", || async {
        let rows = rig.server_rows(BOT_STREAM_ID).await?;
        let local = local_bot_records(&outcome.client).await?;
        Ok(rows.len() == total && same_as_server(&local, &rows))
    })
    .await
}

struct Kept {
    outcome: Outcome,
    tap: Tap,
}

async fn run_variant(name: &str, damage: Damage, queue: bool) -> Result<(Rig, Kept), String> {
    let rig = rig(name).await?;
    let tap = Tap::start(rig.served.origin_addr()).await?;
    let dir = test_harness::test_dir(&format!("{name}_client"))?;
    queued_while_offline(&rig, &tap, &dir, queue).await?;
    let outcome = reopen_after(&rig, &tap, &dir, damage).await?;
    Ok((rig, Kept { outcome, tap }))
}

#[tokio::test]
async fn t_server_n8_1() -> Result<(), String> {
    let (rig, kept) = run_variant("server_n8_1", Damage::DamagedPages, false).await?;
    let outcome = &kept.outcome;
    assert_resynced(&rig, outcome, 3).await?;
    drop(kept.tap);
    drop(kept.outcome);
    rig.served.stop().await
}

#[tokio::test]
async fn a_file_that_is_not_sqlite_is_kept_aside_and_the_outbox_is_reported_unreadable()
-> Result<(), String> {
    let (rig, kept) = run_variant("server_n8_not_sqlite", Damage::NotSqlite, true).await?;
    let outcome = &kept.outcome;
    assert_resynced(&rig, outcome, 3).await?;
    let notice = outcome.client.status().replica_notice.ok_or("no notice")?;
    assert!(notice.outbox_unreadable);
    assert_eq!(notice.rescued_outbox, 0);
    drop(kept.tap);
    drop(kept.outcome);
    rig.served.stop().await
}

#[tokio::test]
async fn a_newer_schema_is_kept_aside_and_the_queued_record_is_rescued_and_sent()
-> Result<(), String> {
    let (rig, kept) = run_variant("server_n8_newer_schema", Damage::NewerSchema, true).await?;
    let outcome = &kept.outcome;
    let notice = outcome.client.status().replica_notice.ok_or("no notice")?;
    assert_eq!(notice.rescued_outbox, 1);
    assert!(!notice.outbox_unreadable);
    assert_resynced(&rig, outcome, 4).await?;
    let rows = rig.server_rows(BOT_STREAM_ID).await?;
    assert_eq!(
        rows.last().map(|row| row.content.clone()),
        Some(message("queued"))
    );
    drop(kept.tap);
    drop(kept.outcome);
    rig.served.stop().await
}

#[tokio::test]
async fn damaged_pages_keep_the_file_aside_and_rescue_what_the_outbox_still_holds()
-> Result<(), String> {
    let (rig, kept) = run_variant("server_n8_damaged_outbox", Damage::DamagedPages, true).await?;
    let outcome = &kept.outcome;
    let notice = outcome.client.status().replica_notice.ok_or("no notice")?;
    assert_eq!(
        notice
            .rescued_outbox
            .saturating_add(notice.unrecovered_outbox),
        1
    );
    assert_resynced(&rig, outcome, 3usize.saturating_add(notice.rescued_outbox)).await?;
    drop(kept.tap);
    drop(kept.outcome);
    rig.served.stop().await
}
