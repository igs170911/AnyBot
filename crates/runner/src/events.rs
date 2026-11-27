use anybot_foundation::{DeviceId, TaskId};
use anybot_protocol::{KnownTaskEventKind, TurnEndReason, TurnInitiator};

use crate::count::thinking_choice_of;
use crate::error::TurnError;
use crate::runner::TurnRunner;

impl TurnRunner {
    pub(crate) fn turn_started_kind(&self, turn: u32, device: DeviceId) -> KnownTaskEventKind {
        KnownTaskEventKind::TurnStarted {
            turn,
            initiator: TurnInitiator::You { device },
            computer: self.computer.clone(),
            model: crate::answer::model_name_of(&self.model.primary.endpoint),
            thinking: thinking_choice_of(self.model.thinking),
        }
    }

    pub(crate) async fn append_turn_end(
        &self,
        task: &TaskId,
        reason: TurnEndReason,
        reply_delivered: bool,
    ) -> Result<(), TurnError> {
        self.append_task_event(
            task,
            KnownTaskEventKind::TurnEnded {
                reason,
                reply_delivered,
            },
        )
        .await
        .map(|_| ())
    }
}
