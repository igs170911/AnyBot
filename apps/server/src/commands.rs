use std::process::ExitCode;

use anybot_foundation::{MasterKey, ServerCommand, ServerLaunchArgs};

use crate::server_run::{Ctx, messages_exit};

pub async fn run_command(
    args: &ServerLaunchArgs,
    version: &str,
    ctx: &mut Ctx,
) -> Option<ExitCode> {
    match args.command.as_ref()? {
        ServerCommand::ResetPassword { account } => {
            Some(crate::cli_reset::reset_main(args, version, account, ctx).await)
        }
        ServerCommand::NewMasterKey => Some(new_master_key(ctx)),
        ServerCommand::ResetTwoStep { account } => {
            Some(crate::cli_two_step::reset_two_step_main(args, version, account, ctx).await)
        }
        ServerCommand::Logs | ServerCommand::UpdateBotComputers | ServerCommand::MigrateOld => None,
    }
}

pub fn new_master_key(ctx: &mut Ctx) -> ExitCode {
    let line = match MasterKey::generate() {
        Ok(key) => key.bootstrap_line(),
        Err(err) => return messages_exit(ctx, &err.to_string()),
    };
    match crate::emit(line.expose(), &mut ctx.stdout) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => messages_exit(ctx, &err.to_string()),
    }
}
