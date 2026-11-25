#[cfg(test)]
use anybot_foundation as _;
use libc as _;
use rustls_pki_types as _;
#[cfg(test)]
use sad_web_push as _;
use serde as _;
#[cfg(test)]
use sqlx as _;
use toml as _;
use url as _;

pub mod cloud_map;
pub mod fake_clock;
pub mod fake_model;
pub mod fake_push;
pub mod http;
pub(crate) mod json_edit;
pub mod launch;
pub mod model_anthropic;
pub mod model_extra;
pub(crate) mod model_hold;
pub mod model_https;
pub mod model_openai;
pub mod model_plan;
pub mod model_script;
pub mod model_tls;
pub mod model_vllm;
pub mod model_vllm_body;
pub mod model_vllm_chunks;
pub mod netctl;
pub(crate) mod netctl_state;
pub mod netproxy;
pub(crate) mod netproxy_flow;
pub mod origin;
pub mod ports;
pub mod postgres;
pub mod process;
pub mod push_crypto;
pub mod realtime;
pub mod running;
pub mod scratch;

pub use cloud_map::{NameMap, cloud_name_matches, mapped_origin};
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
pub use model_hold::arm as arm_model_hold;
pub use model_https::{HttpsModel, https_client, model_start_https, model_start_with_cert};
pub use model_script::{
    ErrorBodyKind, HeaderValueKind, KeyMatch, RecordedRequest, Step, ToolCallScript, UsageScript,
    model_calls_tool, model_cut_off, model_empty_body, model_error_body, model_hold_then_finish,
    model_list, model_list_silent, model_no_response, model_omit_usage, model_redacted_thinking,
    model_redirect, model_replies_text, model_reply_status, model_reply_status_retry, model_stall,
    model_stop_reason, model_stream_error, model_think_then_reply, model_think_then_tool,
    model_thinking_block, model_tool_calls, model_tool_calls_cut, model_tool_calls_length,
    model_usage, random_reply,
};
pub use model_tls::{IssuedLeaf, MintedCert, mint_issued, mint_self_signed};
pub use netctl_state::NetCtl;

pub use origin::{
    collect_delay, collect_stream, first_data_delay, get_via_proxy, origin_url, start_origin,
    start_origin_v6,
};

pub use netctl::{
    NetMode, net_alias, net_attach, net_buffer, net_connect_tunnel, net_cut, net_cut_target,
    net_down, net_hang_connect, net_idle_cut, net_mode, net_restore, net_restore_target, net_slow,
    net_source_addr, net_start,
};
pub use ports::{bind_pair, bind_v4, bind_v6, port_is_free};
pub use postgres::{
    PgInstance, pg_install_cache_dir, pg_migrations_dir, pg_ready, pg_start, pg_stop,
};
pub use push_crypto::{SubscriptionKeys, subscription_endpoint};
pub use realtime::{sleep_real, timeout_real};
pub use running::Running;

pub use scratch::{repo_root, test_dir, write_file_with_mode};
