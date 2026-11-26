use std::sync::Arc;

use anybot_domain;
use anybot_foundation::{Clock, SystemClock};
use anybot_server::{Migrator, boot_database, open_url, product_migrations};
use test_harness::FakeClock;

use crate::serve::{Served, ServedReal, TEST_VERSION, bind_fake};
use crate::serverdb::db_start;

pub async fn serve_start(test: &str) -> Result<Served, String> {
    serve_start_with(test, &anybot_domain::version_release::ReleaseTable::empty()).await
}

pub async fn serve_start_with(
    test: &str,
    releases: &anybot_domain::version_release::ReleaseTable,
) -> Result<Served, String> {
    let db = db_start(test).await?;
    let pool = open_url(&db.url).await.map_err(|e| e.to_string())?;
    let clock = FakeClock::new(1_700_000_000_000);
    let clock_dyn: Arc<dyn Clock> = Arc::clone(&clock) as Arc<dyn Clock>;
    let mut migrations: Migrator = product_migrations();
    let started = boot_database(&pool, &*clock_dyn, &mut migrations, TEST_VERSION)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let startup = crate::startup::startup_capture(&started, db.listen_port)?;
    let clock_fast: Arc<FakeClock> = Arc::clone(&clock);
    let clock_arc: Arc<dyn Clock> = clock_fast;
    let config = anybot_server::ServerConfig::from_rows(started.config.rows())?;
    let (server_shutdown, server_tasks, v6_port, state) =
        bind_fake(&db, clock_arc, pool.clone(), config, releases).await?;
    Ok(Served {
        db,
        clock,
        pool,
        started,
        startup,
        state,
        server_shutdown,
        server_tasks,
        v6_port,
    })
}

pub async fn serve_start_real(test: &str) -> Result<ServedReal, String> {
    let db = db_start(test).await?;
    let pool = open_url(&db.url).await.map_err(|e| e.to_string())?;
    let clock = Arc::new(SystemClock);
    let mut migrations: Migrator = product_migrations();
    let started = boot_database(&pool, clock.as_ref(), &mut migrations, TEST_VERSION)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let clock_arc: Arc<dyn Clock> = clock;
    let config = anybot_server::ServerConfig::from_rows(started.config.rows())?;
    let (server_shutdown, server_tasks, v6_port, state) = bind_fake(
        &db,
        clock_arc,
        pool.clone(),
        config,
        &anybot_domain::version_release::ReleaseTable::empty(),
    )
    .await?;
    Ok(ServedReal {
        db,
        pool,
        started,
        state,
        server_shutdown,
        server_tasks,
        v6_port,
    })
}
