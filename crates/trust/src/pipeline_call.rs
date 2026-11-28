use anybot_foundation::{CallId, TurnId};

use crate::action::GateAction;
use crate::pipeline::ActionRequest;
use crate::runner::GatedComputer;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallContext {
    pub call: CallId,
    pub turn: TurnId,
    pub provider: String,
}

impl CallContext {
    #[must_use]
    pub fn new(call: CallId, turn: TurnId, provider: &str) -> Self {
        CallContext {
            call,
            turn,
            provider: provider.to_owned(),
        }
    }
}

pub(crate) fn prepared_action(request: &ActionRequest, computer: &dyn GatedComputer) -> GateAction {
    let mut action = request.gate_action();
    let base = computer.declared_workdir();
    action.workdir = base;
    action.subject =
        crate::path_resolve::resolve_subject(action.subject, computer, &action.workdir);
    action.extra_paths =
        crate::path_resolve::resolved_targets(&request.delete_targets(), computer, &action.workdir);
    action
}
