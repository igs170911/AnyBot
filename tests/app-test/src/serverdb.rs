use std::path::PathBuf;

use anybot_server::{ensure_schema, open_url, product_migrations, set_config_rows};

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
}

pub async fn db_start(test: &str) -> Result<ServerDb, String> {
    let dir = test_dir(test)?;
    let pg = pg_start(&dir).await?;
    let _created = pg.create_database("anybot_test").await;
    let url = pg.url_for("anybot_test");
    let pool = open_url(&url).await.map_err(|e| e.to_string())?;
    let mut migrations = product_migrations();
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
    let bootstrap = dir.join("boot.conf");
    write_file_with_mode(&bootstrap, &format!("{url}\n"), 0o600)?;
    Ok(ServerDb {
        pg,
        dir,
        bootstrap,
        url,
        listen_port,
    })
}

impl ServerDb {
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
