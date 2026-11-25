use anybot_blob::BlobRef;
use anybot_bus::Notification;
use anybot_channels::ChannelHttpClient;
use anybot_domain::TaskStatus;
use anybot_foundation::{Clock, TimestampMs};
use anybot_protocol::RunEvent;

use anybot_store_server::Pool;
use serde_json as _;

pub mod app;
pub mod health;
#[cfg(test)]
mod health_tests;
pub mod openapi;
pub mod startup;

pub use app::{AppState, build_router};
pub use health::{HealthResponse, health};
pub use openapi::ApiDoc;
pub use startup::{DEFAULT_PORT, ServerStartup, bind_listeners, prepare, startup_line};

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
