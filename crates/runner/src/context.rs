use crate::runner::TurnCtx;

impl TurnCtx {
    #[must_use]
    pub(crate) fn peek_evidence(&self) -> u32 {
        let Ok(guard) = self.evidence.try_lock() else {
            return 1;
        };
        *guard
    }

    pub(crate) fn next_evidence(&self) -> u32 {
        let Ok(mut guard) = self.evidence.try_lock() else {
            return 1;
        };
        let value = *guard;
        *guard = value.saturating_add(1);
        value
    }
}

impl Clone for TurnCtx {
    fn clone(&self) -> Self {
        let start = self.peek_evidence();
        TurnCtx {
            task: self.task,
            turn: self.turn,
            turn_id: self.turn_id,
            evidence: tokio::sync::Mutex::new(start),
            tools: self.tools.clone(),
        }
    }
}
