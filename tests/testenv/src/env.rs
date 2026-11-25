use std::sync::Arc;

use axum::Router;
use tokio::net::TcpListener;

use anybot_server::{AppState, build_router};

use test_harness::fake_clock::FakeClock;
use test_harness::fake_model::model_start;
use test_harness::fake_push::push_start;
use test_harness::netctl::net_start;
use test_harness::ports::bind_v4;
use test_harness::realtime::timeout_real;

use crate::control::control_router;
use crate::state::{EnvPorts, TestEnvState};

const START_BUDGET: std::time::Duration = std::time::Duration::from_secs(60);

fn start_server(clock: Arc<FakeClock>) -> Result<u16, String> {
    let (std_listener, port) = bind_v4()?;
    std_listener
        .set_nonblocking(true)
        .map_err(|e| e.to_string())?;
    let listener = TcpListener::from_std(std_listener).map_err(|e| e.to_string())?;
    let app = build_router(AppState::with_clock("testenv-9.9.9", clock));
    tokio::spawn(async move {
        match axum::serve(listener, app).await {
            Ok(()) => {}
            Err(err) => {
                let mut stderr = std::io::stderr();
                let _result = std::io::Write::write_fmt(
                    &mut stderr,
                    format_args!("testenv server stopped{err}\n"),
                );
            }
        }
    });
    Ok(port)
}

pub async fn testenv_start() -> Result<(Arc<TestEnvState>, Router, TcpListener), String> {
    let (net, proxy_port) = timeout_real(net_start(), START_BUDGET).await??;
    let (model, model_port) = timeout_real(model_start(), START_BUDGET).await??;
    let (push, push_port) = timeout_real(push_start(), START_BUDGET).await??;
    let clock = FakeClock::new(1_700_000_000_000);
    let server_port = start_server(Arc::clone(&clock))?;
    let (control_std, control_port) = bind_v4()?;
    control_std
        .set_nonblocking(true)
        .map_err(|e| e.to_string())?;
    let control_listener = TcpListener::from_std(control_std).map_err(|e| e.to_string())?;
    let state = Arc::new(TestEnvState {
        clock,
        model,
        push,
        net,
        ports: EnvPorts {
            server_port,
            model_port,
            push_port,
            proxy_port,
            control_port,
        },
    });
    Ok((
        Arc::clone(&state),
        control_router(Arc::clone(&state)),
        control_listener,
    ))
}
