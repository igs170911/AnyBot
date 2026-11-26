use crate::runner::{TurnCtx, TurnRunner};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Proceed {
    Go,
}

pub(crate) fn run_checks(_runner: &TurnRunner, _ctx: &TurnCtx) -> Proceed {
    Proceed::Go
}
