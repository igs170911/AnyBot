use std::path::PathBuf;
use std::sync::Arc;

use anybot_foundation::MasterKey;
use anybot_server::{
    Migrator, SecretKeys, ensure_schema, open_url, product_migrations, set_config_rows,
};

use test_harness::{
    PgInstance, pg_start, pg_stop, ports::free_dual_port, scratch::test_dir,
    scratch::write_file_with_mode,
};

pub struct ServerDb {
    pub pg: PgInstance,
    pub dir: PathBuf,
    pub bootstrap: PathBuf,
    pub url: String,
    pub listen_port: u16,
    pub master_key: MasterKey,
}

pub async fn db_start(test: &str) -> Result<ServerDb, String> {
    db_start_with(test, product_migrations()).await
}

pub async fn db_start_with(test: &str, mut migrations: Migrator) -> Result<ServerDb, String> {
    let dir = test_dir(test)?;
    let pg = pg_start(&dir).await?;
    let _created = pg.create_database("anybot_test").await;
    let url = pg.url_for("anybot_test");
    let pool = open_url(&url).await.map_err(|e| e.to_string())?;
    ensure_schema(&pool, &mut migrations)
        .await
        .map_err(|e| format!("fixture migrate rejected: {e}"))?;
    let listen_port = free_dual_port()?;
    set_config_rows(
        &pool,
        &[
            ("listen_addresses".to_owned(), "127.0.0.1,::1".to_owned()),
            ("listen_port".to_owned(), listen_port.to_string()),
        ],
        &[],
    )
    .await
    .map_err(|e| e.to_string())?;
    let master_key = MasterKey::generate().map_err(|e| e.to_string())?;
    let bootstrap = dir.join("boot.conf");
    write_file_with_mode(&bootstrap, &bootstrap_text(&url, &master_key), 0o600)?;
    Ok(ServerDb {
        pg,
        dir,
        bootstrap,
        url,
        listen_port,
        master_key,
    })
}

#[must_use]
pub fn bootstrap_text(url: &str, master_key: &MasterKey) -> String {
    format!("database={url}\n{}\n", master_key.bootstrap_line().expose())
}

impl ServerDb {
    pub fn secret_keys(&self) -> Result<Arc<SecretKeys>, String> {
        SecretKeys::from_master(&self.master_key)
            .map(Arc::new)
            .map_err(|e| e.to_string())
    }

    #[must_use]
    pub fn args(&self) -> Vec<String> {
        vec![
            "--bootstrap".to_owned(),
            self.bootstrap.display().to_string(),
        ]
    }

    pub async fn stop(&self) -> Result<(), String> {
        pg_stop(&self.pg).await
    }
}
