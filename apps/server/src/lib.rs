#[cfg(test)]
use anybot_app_test as _;
use sqlx as _;
use std::process::ExitCode;
#[cfg(test)]
use test_harness as _;
#[cfg(test)]
use tracing as _;

use tokio::net::TcpListener;

use anybot_foundation::{FoundationError, ServerLaunchArgs};
use anybot_messages::{Catalog, MessageCode, MessagesError, ParamValue, Params};

use anybot_server::serve_all::{serve_all, serve_all_stoppable};
pub use anybot_server::startup::ServerStartup as AppServerStartup;
use anybot_server::startup::{DEFAULT_PORT, ServerStartup, bind_addrs, prepare};
use clap::{CommandFactory, Parser};

use axum as _;

pub mod bridge;
pub mod cli_reset;
pub mod line_out;
pub mod server_run;

pub const CODE_SERVER_LISTEN_ADDRESS_INVALID: &str = "server-listen-address-invalid";
pub const CODE_SERVER_STARTUP_NO_ACCOUNTS: &str = "server-startup-no-accounts";
pub const CODE_SERVER_STARTUP_INVITE_GENERATED: &str = "server-startup-invite-generated";
pub const CODE_SERVER_DB_CONNECT_RETRY: &str = "server-db-connect-retry";
pub const CODE_SERVER_DB_FOREIGN_DATA: &str = "server-db-foreign-data";
pub const CODE_SERVER_MIGRATION_FAILED: &str = "server-migration-failed";

pub use bridge::{foundation_message, params_from_ref};
pub use line_out::{FailureLine, Line, StartupLine, StopLine, emit, line_text};
pub use server_run::Ctx;

#[derive(Debug)]
pub struct PlanFailure {
    pub code: MessageCode,
    pub params: Params,
    pub exit: ExitCode,
}

impl PlanFailure {
    #[must_use]
    pub fn new(code: MessageCode, params: Params, exit: u8) -> Self {
        PlanFailure {
            code,
            params,
            exit: ExitCode::from(exit),
        }
    }

    pub fn from_error(error: &FoundationError, exit: u8) -> Result<Self, MessagesError> {
        let (code, params) = foundation_message(error)?;
        Ok(PlanFailure {
            code,
            params,
            exit: ExitCode::from(exit),
        })
    }

    pub fn render(&self, catalog: &mut Catalog) -> Result<String, MessagesError> {
        line_text(catalog, self)
    }
}

impl Line for PlanFailure {
    fn code(&self) -> MessageCode {
        self.code
    }

    fn params(&self) -> &Params {
        &self.params
    }
}

#[must_use]
pub fn help_text() -> String {
    ServerLaunchArgs::command().render_help().to_string()
}

#[derive(Debug)]
pub enum Parsed {
    Args(ServerLaunchArgs),
    Printed(String),
}

pub fn parse_launch(argv: &[String]) -> Result<Parsed, PlanFailure> {
    let mut full = vec!["anybot-server".to_owned()];
    full.extend_from_slice(argv);
    match ServerLaunchArgs::try_parse_from(full) {
        Ok(launch) => Ok(Parsed::Args(launch)),
        Err(err) => {
            let kind = err.kind();
            if kind == clap::error::ErrorKind::DisplayHelp
                || kind == clap::error::ErrorKind::DisplayVersion
            {
                return Ok(Parsed::Printed(err.to_string()));
            }
            Err(PlanFailure {
                code: MessageCode::ServerArgumentsInvalid,
                params: Params::single("reason", ParamValue::text(&err.to_string())),
                exit: ExitCode::from(2),
            })
        }
    }
}

pub fn startup_plan(
    args: &ServerLaunchArgs,
    version: &str,
) -> Result<Result<ServerStartup, PlanFailure>, MessagesError> {
    match prepare(args, version) {
        Ok(startup) => Ok(Ok(startup)),
        Err(err) => Ok(Err(PlanFailure::from_error(&err, 2)?)),
    }
}

pub async fn bind_plan(
    addresses: &[std::net::SocketAddr],
) -> Result<Vec<TcpListener>, PlanFailure> {
    let port = addresses
        .first()
        .map_or(DEFAULT_PORT, std::net::SocketAddr::port);
    bind_addrs(addresses).await.map_err(|err| {
        PlanFailure::new(
            MessageCode::ServerPortUnusable,
            Params::new()
                .with_int("port", i64::from(port))
                .with_text("reason", &err.to_string()),
            1,
        )
    })
}

fn params_map(err: &anybot_messages::ParamsError) -> MessagesError {
    MessagesError::MissingMessage {
        code: format!("params: {err}"),
        lang: Lang::En.tag().to_owned(),
    }
}

use anybot_messages::Lang;

pub fn startup_text(catalog: &mut Catalog, startup: &ServerStartup) -> Result<(), MessagesError> {
    let line = startup_line(startup)?;
    let text = line_text(catalog, &line)?;
    let _result = emit(&text, &mut std::io::stdout());
    Ok(())
}

pub async fn serve_exit(
    catalog: &mut Catalog,
    listeners: Vec<TcpListener>,
    app: axum::Router,
) -> ExitCode {
    let reason = match serve_all(listeners, app).await {
        Ok(reason) | Err(reason) => reason,
    };
    match stop_line(&reason) {
        Ok(line) => stop_exit(catalog, &line),
        Err(err) => {
            let _result = emit(&err.to_string(), &mut std::io::stderr());
            ExitCode::from(3)
        }
    }
}

pub async fn serve_exit_stoppable(
    catalog: &mut Catalog,
    listeners: Vec<TcpListener>,
    app: axum::Router,
    shutdown: std::sync::Arc<tokio::sync::Notify>,
) -> ExitCode {
    let reason = match serve_all_stoppable(listeners, app, shutdown).await {
        Ok(reason) | Err(reason) => reason,
    };
    match stop_line(&reason) {
        Ok(line) => stop_exit(catalog, &line),
        Err(err) => {
            let _result = emit(&err.to_string(), &mut std::io::stderr());
            ExitCode::from(3)
        }
    }
}

pub fn stop_exit(catalog: &mut Catalog, line: &StopLine) -> ExitCode {
    let text = match line_text(catalog, line) {
        Ok(text) => text,
        Err(err) => {
            let _result = emit(&err.to_string(), &mut std::io::stderr());
            return ExitCode::from(3);
        }
    };
    if line.code == StopLine::CODE_NORMAL {
        let _result = emit(&text, &mut std::io::stdout());
        return ExitCode::SUCCESS;
    }
    let _result = emit(&text, &mut std::io::stderr());
    ExitCode::from(1)
}

pub fn report_failure(
    catalog: &mut Catalog,
    failure: &PlanFailure,
    out: &mut impl Write,
) -> ExitCode {
    let text = match failure.render(catalog) {
        Ok(text) => text,
        Err(err) => {
            let _result = emit(&err.to_string(), &mut std::io::stderr());
            return ExitCode::from(3);
        }
    };
    let _result = emit(&text, out);
    failure.exit
}

use std::io::Write;

pub fn startup_line(startup: &ServerStartup) -> Result<StartupLine, MessagesError> {
    Ok(StartupLine::new(
        params_from_ref(&startup.startup).map_err(|e| params_map(&e))?,
    ))
}

pub fn stop_line(reason: &anybot_foundation::MessageRef) -> Result<StopLine, MessagesError> {
    let code = bridge::code_from_str(&reason.code)?;
    Ok(StopLine::new(
        code,
        params_from_ref(reason).map_err(|e| params_map(&e))?,
    ))
}

#[must_use]
pub fn subcommand_plan(args: &ServerLaunchArgs) -> Option<PlanFailure> {
    args.command.as_ref().map(|command| {
        PlanFailure::new(
            MessageCode::ServerSubcommandUnimplemented,
            Params::single("command", ParamValue::text(&format!("{command:?}"))),
            2,
        )
    })
}

#[must_use]
pub fn failure_line(failure: &PlanFailure) -> FailureLine {
    FailureLine::new(failure.code, failure.params.clone())
}
