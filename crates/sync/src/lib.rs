pub use anybot_foundation::Secret;

#[cfg(test)]
use proptest as _;
#[cfg(test)]
use test_harness as _;

pub mod action;
pub mod after_merge;
pub mod client;
mod client_settings;
pub mod config;
mod core;
pub mod error;
mod flush;
pub mod heartbeat_idle;
pub mod heartbeat_keepalive;
pub mod hooks;
mod http_gate;
mod http_phase;
pub mod http_wire;
pub mod idem;
mod link;
mod link_task;
pub mod outbox;
mod pull;
pub mod reconnect_connect;
pub mod reconnect_direct;
pub mod reconnect_http;
pub mod reconnect_pacing;
pub mod reconnect_random;
pub mod reconnect_status;
mod reply_gate;
mod resync;
mod setting_hold;
pub mod sse_hint;
pub mod sse_parse;
pub mod sse_session;
pub mod status;
mod wanted;
pub mod wire;
mod worker;

pub use action::{ACTION_UNREACHABLE_CODE, ActionError, ActionReply};
pub use after_merge::{Replica, StreamDelta, StreamItem, delta_from};
pub use client::{NewRecord, StreamTarget, SyncClient};
pub use config::SyncConfig;
pub use error::SyncError;
pub use hooks::{StreamMark, SyncHooks};
pub use outbox::{RETRY_CAP_SECONDS, retry_delay_secs, retry_schedule};
pub use reconnect_random::{OsRandom, RandomSource, RandomUnavailable, SeededRandom};
pub use status::{ConnectionState, ReplicaNotice, ResyncNotice, SyncStatus};
pub use wire::{Wire, WireFailure, WireFuture, WireLimits, WireMethod, WireReply, WireRequest};

#[cfg(test)]
mod action_tests;
#[cfg(test)]
mod after_merge_tests;
#[cfg(test)]
mod fake_wire;
#[cfg(test)]
mod flush_tests;
#[cfg(test)]
mod outbox_tests;
#[cfg(test)]
mod pull_tests;
#[cfg(test)]
mod setting_tests;
#[cfg(test)]
mod sync_id_tests;
