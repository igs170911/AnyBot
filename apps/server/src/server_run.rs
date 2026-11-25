use std::process::ExitCode;
use std::sync::Arc;
use std::time::Duration;

use tokio::net::TcpListener;

use anybot_foundation::{Clock, MessageRef, ServerBootstrap, ServerLaunchArgs, SystemClock};
use anybot_messages::{Catalog, Lang};
use anybot_server::config::ServerConfig;
use anybot_server::run::{boot_database, connect_retry_line};
use anybot_server::startup::parse_listen_host;
use anybot_server::startup::startup_message;
use anybot_server::{
    AppState, Pool, Started, build_router, open_lazy, probe, probe_pool, product_migrations,
};

use crate::bridge::{code_from_str, params_from_ref};
use crate::line_out::{FailureLine, StartupLine, emit, line_text_in};
use crate::{AppServerStartup, PlanFailure, bind_plan, report_failure, serve_exit_stoppable};

pub const EXIT_SCHEMA: u8 = 2;

pub struct Ctx {
    pub catalog: Catalog,
    pub lang: Lang,
    pub stdout: Box<dyn std::io::Write + Send>,
    pub stderr: Box<dyn std::io::Write + Send>,
}

impl Default for Ctx {
    fn default() -> Self {
        Self::new()
    }
}

impl Ctx {
    #[must_use]
    pub fn new() -> Self {
        Ctx {
            catalog: Catalog::new(),
            lang: Lang::En,
            stdout: Box::new(std::io::stdout()),
            stderr: Box::new(std::io::stderr()),
        }
    }

    pub fn set_lang(&mut self, text: &str) {
        self.lang = Lang::parse(text).unwrap_or(Lang::En);
    }

    pub fn print_ref(&mut self, reference: &MessageRef) -> Result<(), String> {
        let code = code_from_str(&reference.code).map_err(|e| e.to_string())?;
        let params = params_from_ref(reference).map_err(|e| e.to_string())?;
        let line = FailureLine::new(code, params);
        let text = line_text_in(&mut self.catalog, self.lang, &line).map_err(|e| e.to_string())?;
        emit(&text, &mut self.stdout).map_err(|e| e.to_string())
    }

    pub fn report(&mut self, failure: &PlanFailure) -> ExitCode {
        report_failure(&mut self.catalog, failure, &mut self.stderr)
    }

    pub fn startup_line(&mut self, reference: &MessageRef) -> Result<(), String> {
        let params = params_from_ref(reference).map_err(|e| e.to_string())?;
        let line = StartupLine::new(params);
        let text = line_text_in(&mut self.catalog, self.lang, &line).map_err(|e| e.to_string())?;
        emit(&text, &mut self.stdout).map_err(|e| e.to_string())
    }
}

pub async fn connect_plan(
    bootstrap: &ServerBootstrap,
    ctx: &mut Ctx,
    clock: &dyn Clock,
) -> Result<Pool, String> {
    loop {
        let gate = probe_pool(bootstrap);
        match probe(&gate).await {
            Ok(_one) => {
                let _closed = gate.close().await;
                return Ok(open_lazy(bootstrap));
            }
            Err(err) => {
                let _closed = gate.close().await;
                let reference = connect_retry_line(bootstrap, &err.to_string());
                ctx.print_ref(&reference)?;
                let _result = clock.sleep(Duration::from_secs(5)).await;
            }
        }
    }
}

pub async fn boot_plan(
    pool: &Pool,
    version: &str,
    bootstrap: &ServerBootstrap,
    ctx: &mut Ctx,
) -> Result<Started, ExitCode> {
    let clock = SystemClock;
    let mut migrations = product_migrations();
    let outcome = boot_database(pool, &clock, &mut migrations, version).await;
    match outcome {
        Ok(started) => Ok(started),
        Err(err) => {
            if ctx.print_ref(&err.log_line(bootstrap)).is_err() {
                return Err(ExitCode::from(3));
            }
            Err(ExitCode::from(EXIT_SCHEMA))
        }
    }
}

pub async fn bind_from_config(
    config: &ServerConfig,
    ctx: &mut Ctx,
) -> Result<Vec<TcpListener>, ExitCode> {
    let mut addrs = Vec::new();
    for text in &config.listen_addresses {
        match parse_listen_host(text, config.listen_port) {
            Ok(addr) => addrs.push(addr),
            Err(reason) => {
                if ctx.print_ref(&listen_invalid_line(text, &reason)).is_err() {
                    return Err(ExitCode::from(3));
                }
                return Err(ExitCode::from(1));
            }
        }
    }
    match bind_plan(&addrs).await {
        Ok(listeners) => Ok(listeners),
        Err(failure) => Err(ctx.report(&failure)),
    }
}

fn listen_invalid_line(text: &str, reason: &str) -> MessageRef {
    MessageRef::new(crate::CODE_SERVER_LISTEN_ADDRESS_INVALID)
        .with_text("address", text)
        .with_text("reason", reason)
}

#[must_use]
pub fn startup_reference(
    startup: &AppServerStartup,
    config: &ServerConfig,
    version: &str,
) -> MessageRef {
    startup_message(
        version,
        &config.listen_addresses,
        config.listen_port,
        &startup.bootstrap,
    )
}

pub fn note_startup_logs(ctx: &mut Ctx, started: &Started) {
    if started.account_count == 0 {
        let reference = MessageRef::new(crate::CODE_SERVER_STARTUP_NO_ACCOUNTS);
        let _printed = ctx.print_ref(&reference);
    }
    if let Some(code) = &started.new_invite {
        let reference =
            MessageRef::new(crate::CODE_SERVER_STARTUP_INVITE_GENERATED).with_text("code", code);
        let _printed = ctx.print_ref(&reference);
    }
}

pub async fn serve_main(args: &ServerLaunchArgs, version: &str, ctx: &mut Ctx) -> ExitCode {
    let startup = match crate::startup_plan(args, version) {
        Ok(Ok(startup)) => startup,
        Ok(Err(failure)) => return ctx.report(&failure),
        Err(err) => return messages_exit(ctx, &err.to_string()),
    };
    let clock = Arc::new(SystemClock);
    let pool = match connect_plan(&startup.bootstrap, ctx, clock.as_ref()).await {
        Ok(pool) => pool,
        Err(err) => return messages_exit(ctx, &err),
    };
    let started = match boot_plan(&pool, version, &startup.bootstrap, ctx).await {
        Ok(started) => started,
        Err(code) => return code,
    };
    ctx.set_lang(&started.config.language);
    let reference = startup_reference(&startup, &started.config, version);
    let listeners = match bind_from_config(&started.config, ctx).await {
        Ok(listeners) => listeners,
        Err(code) => return code,
    };
    if ctx.startup_line(&reference).is_err() {
        return ExitCode::from(3);
    }
    note_startup_logs(ctx, &started);
    let clock = Arc::new(SystemClock);
    let state = AppState::with_pool(version, clock, pool, started.config);
    let shutdown = Arc::clone(&state.shutdown);
    let app = build_router(state);
    serve_exit_stoppable(&mut ctx.catalog, listeners, app, shutdown).await
}

pub fn messages_exit(ctx: &mut Ctx, err: &str) -> ExitCode {
    let _result = emit(err, &mut ctx.stderr);
    ExitCode::from(3)
}
