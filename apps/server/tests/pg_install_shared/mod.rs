use tokio as _;

use sqlx as _;

use anybot_app_server as _;
use anybot_foundation as _;
use anybot_messages as _;
use anybot_server as _;
use anybot_store_server as _;
use axum as _;
use clap as _;
use rpassword as _;
use serde_json as _;
use tracing as _;

pub(crate) use anybot_app_test::{ServerDb, ServerProgram, db_start, english};

use anybot_messages::{Catalog, MessageCode};

pub(crate) fn server_bin() -> ServerProgram {
    ServerProgram::new(env!("CARGO_BIN_EXE_anybot-server"))
}

pub(crate) fn startup_line_of(db: &ServerDb) -> Result<String, String> {
    english(
        &mut Catalog::new(),
        MessageCode::ServerStartupListening,
        &anybot_app_test::startup_params(env!("CARGO_PKG_VERSION"), db.listen_port),
    )
}
