use anybot_blob::BlobRef;
use anybot_bus::Notification;
use anybot_channels::ChannelHttpClient;
use anybot_domain::TaskStatus;
use anybot_foundation::{Clock, TimestampMs};
use anybot_protocol::RunEvent;
use serde_json as _;

pub mod account_input;
pub mod api_account;
pub mod api_admin;
pub mod api_admin_accounts;
pub mod api_admin_reset;
pub mod api_computers;
pub mod api_computers_act;
pub mod api_login_computer;
pub mod api_login_flow;
pub mod api_login_issue;
pub mod api_mailbox;
pub mod api_ops;
pub mod api_ops_session;
pub mod api_pairing;
pub mod api_pairing_redeem;
pub mod api_referee;
pub mod api_referee_card;
pub mod api_register;
pub mod api_settings;
pub mod api_settings_write;
pub mod api_streams_action;
pub mod api_streams_action_plan;
pub mod api_streams_after;
pub mod api_streams_append;
pub mod api_streams_bot;
pub mod api_streams_dedup;
pub mod api_streams_record;
pub mod api_streams_sse;
pub mod api_streams_sse_drive;
pub mod api_streams_util;
pub mod api_two_step_login;
pub mod api_two_step_setup;
pub mod api_two_step_toggle;
pub mod app;
pub mod auth_credential;
pub mod auth_csrf;
pub mod auth_pre;
pub mod auth_time;
pub mod auth_version;
pub mod boot;
pub mod boot_config;
pub mod caller;
pub mod caller_device;
pub mod caller_session;
pub mod cleanup;
pub mod computer_view;
pub mod computers_action;
pub mod computers_join;
pub mod computers_log;
pub mod computers_remove;
pub mod computers_routes;
pub mod computers_service;
pub mod config;
pub mod conn_limit;
pub mod device_notes;
pub mod health;
pub mod hex;
pub mod idem;
pub mod invite;
pub mod limits;
pub mod limits_pairing;
pub mod mailbox_card;
pub mod mailbox_claim;
pub mod mailbox_deliver;
pub mod mailbox_reassign;
pub mod mailbox_removal;
pub mod mailbox_routes;
pub mod mailbox_sse;
pub mod mailbox_sweep;
pub mod openapi;
pub mod pairing_issue;
pub mod pairing_redeem;
pub mod pass_hash;
pub mod presence;
pub mod presence_sweep;
pub mod problems;
pub mod reauth_state;
pub mod req_body_limit;
pub mod req_source;
pub mod req_write;
pub mod run;
pub mod runner_device;
pub mod serve_all;
pub mod session_key;
pub mod sse_revoke;
pub mod startup;
pub mod status;
pub mod two_step_challenge;
pub mod two_step_check;
pub mod two_step_gate;
pub mod two_step_login_flow;

pub use account_input::{normalize_account, password_long_enough};
pub use anybot_store_server::SecretKeys;
pub use anybot_store_server::{
    Migrator, Pool, SchemaError, ensure_schema, max_applied_version, min_applied_version,
    open_lazy, open_url, probe, probe_pool, product_migrations, read_config_rows, set_config_rows,
    write_config_row,
};
pub use app::{AppState, build_router};
pub use boot::{BootError, Started, boot};
pub use boot_config::load_config;
pub use config::{
    KEY_INVITE_CODE, KEY_LANGUAGE, KEY_LISTEN_ADDRESSES, KEY_LISTEN_PORT, KEY_PROXY_SECRET,
    KEY_PUBLIC_URL, KEY_SERVER_VERSION, KEY_TRUST_PROXY, ServerConfig,
};
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
