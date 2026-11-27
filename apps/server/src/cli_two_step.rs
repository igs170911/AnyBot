use std::process::ExitCode;

use anybot_foundation::{MessageRef, ServerLaunchArgs, SystemClock};
use anybot_server::normalize_account;
use anybot_store_server::{Pool, accounts, two_step};

use crate::Ctx;
use crate::cli_reset::schema_gate;
use crate::line_out::emit;
use crate::server_run::{connect_plan, messages_exit};

pub const CODE_TWO_STEP_RESET_DONE: &str = "server-two-step-reset-done";

pub async fn reset_two_step_main(
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
    let pool = match connect_plan(&startup.bootstrap, ctx, &SystemClock).await {
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
    match clear_account(&pool, &account).await {
        Ok(()) => done(ctx, &account),
        Err(reason) => {
            let _printed = emit(&reason, &mut ctx.stderr);
            ExitCode::from(3)
        }
    }
}

async fn clear_account(pool: &Pool, account: &str) -> Result<(), String> {
    let found = accounts::find_account(pool, account)
        .await
        .map_err(|e| e.to_string())?;
    if found.is_none() {
        return Err(format!("account not found: {account}"));
    }
    two_step::clear_two_step(pool, account)
        .await
        .map_err(|e| e.to_string())
}

fn done(ctx: &mut Ctx, account: &str) -> ExitCode {
    let reference = MessageRef::new(CODE_TWO_STEP_RESET_DONE).with_text("account", account);
    match ctx.print_ref(&reference) {
        Ok(()) => ExitCode::SUCCESS,
        Err(reason) => messages_exit(ctx, &reason),
    }
}
