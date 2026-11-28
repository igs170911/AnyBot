use serde::Serialize;

use test_harness::NetCtl;
use test_harness::PgInstance;
use test_harness::fake_clock::FakeClock;
use test_harness::fake_model::FakeModel;
use test_harness::fake_push::FakePush;

use std::sync::Arc;

use crate::locks::LockTable;
use crate::seed::Admin;
use test_harness::server_client::ServerApi;

#[derive(Clone, Serialize)]
pub struct EnvPorts {
    pub server_port: u16,
    pub model_port: u16,
    pub push_port: u16,
    pub proxy_port: u16,
    pub control_port: u16,
}

pub struct TestEnvState {
    pub clock: Arc<FakeClock>,
    pub model: Arc<FakeModel>,
    pub push: Arc<FakePush>,
    pub net: Arc<NetCtl>,
    pub pg: PgInstance,
    pub api: ServerApi,
    pub admin: Admin,
    pub locks: LockTable,
    pub ports: EnvPorts,
}

#[must_use]
pub fn advance_ms(state: &TestEnvState, ms: u64) -> usize {
    test_harness::fake_clock::advance_clock(&state.clock, std::time::Duration::from_millis(ms))
}
