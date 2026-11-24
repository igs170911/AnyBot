use anybot_blob::BlobRef;
use anybot_bus::Notification;
use anybot_channels::ChannelHttpClient;
use anybot_domain::TaskStatus;
use anybot_foundation::now_epoch_seconds;
use anybot_protocol::EventEnvelope;
use anybot_store_server::Pool;

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
    pub event: EventEnvelope,
    pub blob: Option<BlobRef>,
    pub at_epoch_seconds: i64,
}

impl ServerEventBundle {
    #[must_use]
    pub fn new(event: EventEnvelope) -> Self {
        ServerEventBundle {
            event,
            blob: None,
            at_epoch_seconds: now_epoch_seconds(),
        }
    }
}
