use anybot_blob::BlobRef;
use anybot_bus::Notification;
use anybot_channels::ChannelHttpClient;
use anybot_domain::TaskStatus;
use anybot_foundation::{Clock, TimestampMs};
use anybot_protocol::RunEvent;
use serde_json as _;

pub mod app;
pub mod boot;
pub mod config;
pub mod health;
pub mod invite;
pub mod openapi;
pub mod run;
pub mod serve_all;
pub mod startup;
pub mod status;

pub use anybot_store_server::{
    Migrator, Pool, SchemaError, ensure_schema, open_lazy, open_url, probe, probe_pool,
    product_migrations, read_config_rows, set_config_rows, write_config_row,
};
pub use app::{AppState, build_router};
pub use boot::{BootError, Started, boot};
pub use config::ServerConfig;
pub use health::{HealthResponse, health};
pub use invite::draw_invite;
pub use openapi::ApiDoc;
pub use run::{RunError, boot_database, connect_retry_line};
pub use startup::{
    DEFAULT_PORT, ServerStartup, bind_addrs, bind_listeners, parse_listen_host, prepare,
    startup_message,
};
#[cfg(test)]
use test_harness as _;
#[cfg(test)]
use time as _;

#[derive(Debug)]
pub struct AppStateRef {
    pub status: TaskStatus,
}

impl AppStateRef {
    #[must_use]
    pub fn example() -> Self {
        AppStateRef {
            status: TaskStatus::Running,
        }
    }
}

pub type ServerPool = Pool;
pub type ServerNotification = Notification;
pub type ServerChannelClient = ChannelHttpClient;

#[derive(Debug)]
pub struct ServerEventBundle {
    pub event: RunEvent,
    pub blob: Option<BlobRef>,
    pub at: TimestampMs,
}

impl ServerEventBundle {
    #[must_use]
    pub fn new(event: RunEvent, clock: &dyn Clock) -> Self {
        ServerEventBundle {
            event,
            blob: None,
            at: clock.now_ms(),
        }
    }
}
