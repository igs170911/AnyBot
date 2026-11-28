use crate::error::TurnError;
use crate::record::Halt;
use crate::runner::{TurnCtx, TurnRunner};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Proceed {
    Go,
    Halted(Halt),
}

pub(crate) async fn run_checks(runner: &TurnRunner, ctx: &TurnCtx) -> Result<Proceed, TurnError> {
    if let Some(halt) = crate::halt::check_halt(runner, ctx).await? {
        return Ok(Proceed::Halted(halt));
    }
    Ok(Proceed::Go)
}
