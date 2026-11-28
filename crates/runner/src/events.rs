use anybot_protocol::{KnownTaskEventKind, TurnEndReason, TurnInitiator};

use crate::count::thinking_choice_of;
use crate::error::TurnError;
use crate::runner::{TurnCtx, TurnRunner};

impl TurnRunner {
    pub(crate) fn turn_started_kind(
        &self,
        turn: u32,
        initiator: TurnInitiator,
    ) -> KnownTaskEventKind {
        KnownTaskEventKind::TurnStarted {
            turn,
            initiator,
            computer: self.computer.clone(),
            model: crate::answer::model_name_of(&self.model.primary.endpoint),
            thinking: thinking_choice_of(self.model.thinking),
        }
    }

    pub(crate) async fn append_turn_end(
        &self,
        ctx: &TurnCtx,
        reason: TurnEndReason,
        reply_delivered: bool,
    ) -> Result<(), TurnError> {
        self.append_turn_event(
            ctx,
            KnownTaskEventKind::TurnEnded {
                reason: reason.clone(),
                reply_delivered,
            },
        )
        .await?;
        if reason != TurnEndReason::Interrupted {
            self.record
                .trim_turn(ctx.task, ctx.turn)
                .await
                .map_err(TurnError::Record)?;
        }
        Ok(())
    }
}
