#[cfg(test)]
use anybot_foundation as _;
use libc as _;
#[cfg(test)]
use sad_web_push as _;
use serde as _;
#[cfg(test)]
use sqlx as _;
use toml as _;
use url as _;

pub fn test_tracing() {
    use std::sync::Once;
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        let _set = tracing_subscriber::fmt()
            .with_max_level(tracing::Level::DEBUG)
            .try_init();
    });
}

pub mod fake_clock;
pub mod fake_model;
pub mod fake_push;
pub mod http;
pub mod launch;
pub mod model_anthropic;
pub mod model_openai;
pub mod model_plan;
pub mod model_script;
pub mod model_vllm;
pub mod netctl;
pub(crate) mod netctl_state;
pub mod netproxy;
pub(crate) mod netproxy_flow;
pub mod netproxy_raw;
pub mod origin;
pub mod ports;
pub mod postgres;
pub mod process;
pub mod push_crypto;
pub mod realtime;
pub mod running;
pub mod scratch;
pub mod serverdb;

pub use fake_clock::{
    ClockGone, FakeClock, advance_clock, sleep_for, sleep_until, timeout as fake_clock_timeout,
};
pub use fake_model::{FakeModel, ModelFormat, model_start, model_start_with_format};
pub use fake_push::{FakePush, PushRecord, push_start};
pub use http::{
    http_get, http_get_addr, http_send_get, http_wait, plain_client, request, sse_lines, sse_post,
    sse_post_client, sse_send, sse_wait_for_line, test_delete, test_get_async, test_post, test_put,
};
pub use launch::{
    Outcome, Program, child_is_dead, run_program, run_program_clocked, run_program_with,
};
pub use model_script::{
    HeaderValueKind, RecordedRequest, Step, model_calls_tool, model_cut_off, model_no_response,
    model_replies_text, model_reply_status, model_stall, model_think_then_reply,
};
pub use netctl_state::NetCtl;

pub use origin::{
    collect_delay, collect_stream, first_data_delay, get_via_proxy, origin_url, start_origin,
    start_origin_v6,
};

pub use netctl::{
    NetMode, net_attach, net_buffer, net_connect_tunnel, net_cut, net_cut_target, net_down,
    net_idle_cut, net_mode, net_probe_open, net_restore, net_restore_target, net_slow,
    net_source_addr, net_start, net_start_raw,
};
pub use ports::{bind_pair, bind_v4, bind_v6, port_is_free};
pub use postgres::{
    PgInstance, pg_install_cache_dir, pg_migrations_dir, pg_ready, pg_start, pg_stop,
};
pub use push_crypto::{SubscriptionKeys, subscription_endpoint};
pub use realtime::{sleep_real, timeout_real};
pub use running::Running;
pub use serverdb::{ServerDb, db_start, migration_fixture};

pub use scratch::{repo_root, test_dir, write_file_with_mode};
