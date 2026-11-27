use anybot_app_server as _;
use anybot_domain as _;
use anybot_foundation as _;
use anybot_messages as _;
use anybot_server as _;
use axum as _;
use clap as _;
use rpassword as _;
use serde_json as _;
use sqlx as _;
use tokio as _;
use tracing as _;

mod pg_mailbox_shared;

use std::io::Write as _;

use anybot_app_test::mailbox_audit::audit;
use anybot_app_test::mailbox_model::{plan, random_seed};
use anybot_app_test::mailbox_prop::Rig;
use anybot_app_test::serve::serve_start;
use pg_mailbox_shared::three_stand_ins;

const REPLAY_SEED: Option<u64> = None;
const REAL_LIMIT: std::time::Duration = std::time::Duration::from_secs(120);

#[tokio::test]
async fn t_mailbox_claim_1() -> Result<(), String> {
    let seed = match REPLAY_SEED {
        Some(fixed) => fixed,
        None => random_seed()?,
    };
    let mut report = std::io::stderr();
    writeln!(report, "T-mailbox-claim-1 seed {seed}").map_err(|e| e.to_string())?;
    let started = test_harness::realtime::instant_now();
    let served = serve_start("t_mailbox_claim_1").await?;
    let seats = three_stand_ins(&served, "bot-prop").await?;
    let built = plan(seed)?;
    let mut rig = Rig::open(&served, seats).await?;
    rig.reconnect_all().await?;
    let outcome = test_harness::timeout_real(async { rig.run(&built).await }, REAL_LIMIT)
        .await
        .map_err(|reason| format!("seed {seed}: {reason}"))?;
    outcome.map_err(|reason| format!("seed {seed}: {reason}"))?;
    rig.settle().await?;
    audit(&rig, &built)
        .await
        .map_err(|reason| format!("seed {seed}: {reason}"))?;
    rig.x.stop();
    writeln!(
        report,
        "T-mailbox-claim-1 took {:?}, {} operations, {} reassigned",
        started.elapsed(),
        built.ops.len(),
        built.moves
    )
    .map_err(|e| e.to_string())?;
    served.stop().await
}
