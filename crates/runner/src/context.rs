use crate::runner::{TurnCtx, TurnModel};

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

    #[must_use]
    pub(crate) fn active_model(&self, fallback: &TurnModel) -> TurnModel {
        let Ok(guard) = self.active_model.lock() else {
            return fallback.copy_model();
        };
        match guard.as_ref() {
            Some(model) => model.copy_model(),
            None => fallback.copy_model(),
        }
    }

    pub(crate) fn set_active_model(&self, model: TurnModel) {
        if let Ok(mut guard) = self.active_model.lock() {
            *guard = Some(model);
        }
    }

    pub(crate) fn advance_evidence_to(&self, min_next: u32) {
        let Ok(mut guard) = self.evidence.try_lock() else {
            return;
        };
        if *guard < min_next {
            *guard = min_next;
        }
    }
}

impl Clone for TurnCtx {
    fn clone(&self) -> Self {
        let start = self.peek_evidence();
        let active = match self.active_model.lock() {
            Ok(guard) => guard.as_ref().map(TurnModel::copy_model),
            Err(_) => None,
        };
        TurnCtx {
            task: self.task,
            turn: self.turn,
            turn_id: self.turn_id,
            turn_started: self.turn_started,
            initiator: self.initiator,
            evidence: tokio::sync::Mutex::new(start),
            active_model: std::sync::Mutex::new(active),
            tools: self.tools.clone(),
        }
    }
}
