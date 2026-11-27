use std::io::IsTerminal;
use std::process::ExitCode;

use rpassword::prompt_password;

use anybot_foundation::{Clock, MessageRef, Secret, ServerLaunchArgs, SystemClock};
use anybot_server::pass_hash::PassHash;
use anybot_server::{normalize_account, password_long_enough};
use anybot_store_server::{Pool, accounts, sessions};

use crate::Ctx;
use crate::line_out::emit;
use crate::server_run::{connect_plan, messages_exit};
use anybot_store_server::max_applied_version;

pub const CODE_CLI_PASSWORD_PROMPT: &str = "server-cli-password-prompt";
pub const CODE_CLI_CONFIRM_PROMPT: &str = "server-cli-password-confirm-prompt";
pub const CODE_CLI_RESET_DONE: &str = "server-cli-reset-password-done";
pub const CODE_CLI_PASSWORD_SHORT: &str = "server-cli-password-short";
pub const CODE_CLI_PASSWORD_MISMATCH: &str = "server-cli-password-mismatch";

#[derive(Debug)]
pub enum ResetFailure {
    Report(String),
    Prompt(String),
    Message(&'static str),
}

pub(crate) async fn schema_gate(pool: &Pool, version: &str, ctx: &mut Ctx) -> Option<ExitCode> {
    let expected = anybot_server::product_migrations()
        .migrations
        .last()
        .map(|migration| migration.version);
    let low = expected?;
    let applied = max_applied_version(pool).await;
    let report = match applied {
        Ok(value) if value == low => return None,
        Ok(value) if value < low => {
            format!("schema too old for version {version}: applied {value}")
        }
        Ok(value) => format!("schema newer than version {version}: applied {value}"),
        Err(err) => format!("schema check failed: {err}"),
    };
    let _printed = emit(&report, &mut ctx.stderr);
    Some(ExitCode::from(3))
}

pub async fn reset_main(
    args: &ServerLaunchArgs,
    version: &str,
    account: &str,
    ctx: &mut Ctx,
) -> ExitCode {
    let startup = match crate::startup_plan(args, version) {
        Ok(Ok(startup)) => startup,
        Ok(Err(failure)) => return ctx.report(&failure),
        Err(err) => return messages_exit(ctx, &err.to_string()),
    };
    let clock = SystemClock;
    let pool = match connect_plan(&startup.bootstrap, ctx, &clock).await {
        Ok(pool) => pool,
        Err(err) => return messages_exit(ctx, &err),
    };
    if let Some(code) = schema_gate(&pool, version, ctx).await {
        return code;
    }
    let account = match normalize_account(account) {
        Ok(name) => name,
        Err(reason) => return messages_exit(ctx, &reason),
    };
    match reset_account(ctx, &pool, &account).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(ResetFailure::Report(reason)) => {
            let _result = emit(&reason, &mut ctx.stderr);
            ExitCode::from(3)
        }
        Err(ResetFailure::Prompt(reason)) => {
            let _printed = emit(&reason, &mut ctx.stderr);
            ExitCode::from(2)
        }
        Err(ResetFailure::Message(code)) => {
            let _printed = ctx.print_ref(&MessageRef::new(code));
            ExitCode::from(2)
        }
    }
}

async fn reset_account(ctx: &mut Ctx, pool: &Pool, account: &str) -> Result<(), ResetFailure> {
    let found = accounts::find_account(pool, account)
        .await
        .map_err(|e| ResetFailure::Report(e.to_string()))?;
    if found.is_none() {
        return Err(ResetFailure::Report(format!(
            "account not found: {account}"
        )));
    }
    let first = read_password(ctx, CODE_CLI_PASSWORD_PROMPT)?;
    let second = read_password(ctx, CODE_CLI_CONFIRM_PROMPT)?;
    validate_passwords(&first, &second)?;
    let hasher = PassHash::new().map_err(ResetFailure::Report)?;
    let stored = hasher
        .hash(&Secret::from_untrusted(first.clone()))
        .await
        .map_err(ResetFailure::Report)?;
    let now_ms = i64::try_from(Clock::now_ms(&SystemClock).millis())
        .map_err(|e| ResetFailure::Report(e.to_string()))?;
    accounts::set_password(pool, account, &stored, now_ms)
        .await
        .map_err(|e| ResetFailure::Report(e.to_string()))?;
    sessions::delete_sessions_for_account(pool, account, None)
        .await
        .map_err(|e| ResetFailure::Report(e.to_string()))?;
    let reference = MessageRef::new(CODE_CLI_RESET_DONE).with_text("account", account);
    ctx.print_ref(&reference).map_err(ResetFailure::Prompt)?;
    Ok(())
}

fn validate_passwords(first: &str, second: &str) -> Result<(), ResetFailure> {
    if !password_long_enough(first) || !password_long_enough(second) {
        return Err(ResetFailure::Message(CODE_CLI_PASSWORD_SHORT));
    }
    if first != second {
        return Err(ResetFailure::Message(CODE_CLI_PASSWORD_MISMATCH));
    }
    Ok(())
}

fn read_password(ctx: &mut Ctx, code: &str) -> Result<String, ResetFailure> {
    let reference = MessageRef::new(code);
    ctx.print_ref(&reference).map_err(ResetFailure::Prompt)?;
    if std::io::stdin().is_terminal() {
        return prompt_password("").map_err(|e| ResetFailure::Report(e.to_string()));
    }
    let mut line = String::new();
    std::io::stdin()
        .read_line(&mut line)
        .map_err(|e| ResetFailure::Report(e.to_string()))?;
    Ok(line.trim_end_matches('\n').to_owned())
}
