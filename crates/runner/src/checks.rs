use anybot_protocol::BudgetItem;

use crate::error::TurnError;
use crate::record::Halt;
use crate::runner::{TurnCtx, TurnRunner};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Proceed {
    Go,
    Halted(Halt),
    OverBudget {
        item: BudgetItem,
        used: u64,
        limit: u64,
    },
}

pub(crate) async fn run_checks(runner: &TurnRunner, ctx: &TurnCtx) -> Result<Proceed, TurnError> {
    if let Some(halt) = crate::halt::check_halt(runner, ctx).await? {
        return Ok(Proceed::Halted(halt));
    }
    if let Some(over) = crate::budget_gate::check_step_budget(runner, ctx).await? {
        return Ok(Proceed::OverBudget {
            item: over.item,
            used: over.used,
            limit: over.limit,
        });
    }
    Ok(Proceed::Go)
}
