use std::path::{Path, PathBuf};

use crate::scratch::test_dir;

use anybot_store_server::{ensure_schema, open_url, product_migrations, set_config_rows};

use crate::postgres::{PgInstance, pg_start, pg_stop};

pub fn migration_fixture(product_dir: &Path, test: &str, extra: &str) -> Result<PathBuf, String> {
    let dir = test_dir(test)?.join("migrations");
    std::fs::create_dir_all(&dir).map_err(|e| format!("create fixture dir failed: {e}"))?;
    for entry in std::fs::read_dir(product_dir)
        .map_err(|e| format!("read product migrations failed: {e}"))?
    {
        let path = entry
            .map_err(|e| format!("fixture entry failed: {e}"))?
            .path();
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_owned();
        std::fs::copy(&path, dir.join(&name)).map_err(|e| format!("fixture copy failed: {e}"))?;
    }
    let sql = if extra.contains("broken") {
        "NOT SQL AT ALL;\n".to_owned()
    } else {
        "CREATE TABLE IF NOT EXISTS fixture_extra (id INTEGER PRIMARY KEY);\n".to_owned()
    };
    std::fs::write(dir.join(extra), sql).map_err(|e| format!("fixture write failed: {e}"))?;
    Ok(dir)
}

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
    let listen_port = crate::ports::free_dual_port()?;
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
    crate::scratch::write_file_with_mode(&bootstrap, &format!("{url}\n"), 0o600)?;
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
