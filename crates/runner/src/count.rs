use anybot_foundation::TaskId;
use anybot_protocol::{KnownTaskEventKind, TaskEventKind};

use crate::error::TurnError;
use crate::runner::{KindCount, TurnRunner};

impl TurnRunner {
    pub(crate) async fn count_task_kind(
        &self,
        task: &TaskId,
        which: KindCount,
    ) -> Result<u32, TurnError> {
        let rows = self
            .record
            .task_events(*task)
            .await
            .map_err(TurnError::Record)?;
        Ok(count_rows(&rows, which))
    }
}

pub(crate) fn count_rows(rows: &[anybot_protocol::TaskEvent], which: KindCount) -> u32 {
    let mut count: u32 = 0;
    for row in rows {
        let TaskEventKind::Known(kind) = &row.kind else {
            continue;
        };
        let matched = matches!(
            (which, kind),
            (
                KindCount::TurnStarted,
                KnownTaskEventKind::TurnStarted { .. }
            ) | (KindCount::ModelAsked, KnownTaskEventKind::ModelAsked { .. })
                | (KindCount::TurnEnded, KnownTaskEventKind::TurnEnded { .. })
        );
        if matched {
            count = count.saturating_add(1);
        }
    }
    count
}

pub(crate) fn thinking_choice_of(
    mode: anybot_models::ThinkingMode,
) -> anybot_protocol::ThinkingChoice {
    match mode {
        anybot_models::ThinkingMode::Auto => anybot_protocol::ThinkingChoice::Auto,
        anybot_models::ThinkingMode::Fast => anybot_protocol::ThinkingChoice::Fast,
        anybot_models::ThinkingMode::Balanced => anybot_protocol::ThinkingChoice::Balanced,
        anybot_models::ThinkingMode::Deep => anybot_protocol::ThinkingChoice::Deep,
    }
}
