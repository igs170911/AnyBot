use crate::collapse::StepView;
use crate::compile::Stage;
use crate::input::{BuildInput, MemoryRecord};
use crate::numbers::K114_4_MIN_VERBATIM_STEPS;

pub(crate) fn drop_oldest_result(views: &[StepView], stage: &mut Stage) -> bool {
    let cutoff = views.len().saturating_sub(K114_4_MIN_VERBATIM_STEPS);
    let Some(id) = views
        .iter()
        .filter(|v| v.index < cutoff && !v.short && !v.keep_full)
        .map(|v| v.source_ref.clone())
        .find(|id| !stage.collapsed.contains(id))
    else {
        return false;
    };
    stage.collapsed.insert(id.clone());
    if !already_collapsed(&id, &stage.events) {
        stage.dropped_results.push(id);
    }
    true
}

fn already_collapsed(id: &str, events: &[crate::output::NewEvent]) -> bool {
    events.iter().any(|event| match event {
        crate::output::NewEvent::Collapsed { ids, .. } => ids.iter().any(|known| known == id),
        crate::output::NewEvent::ContextBuilt | crate::output::NewEvent::MemoryDropped { .. } => {
            false
        }
    })
}

pub(crate) fn drop_oldest_prior(input: &BuildInput, stage: &mut Stage) -> bool {
    let count = input.prior_chat.len();
    if count == 0 || stage.prior_keep == 0 {
        return false;
    }
    let keep = stage.prior_keep.min(count).saturating_sub(1);
    if keep == count {
        return false;
    }
    let split = count.saturating_sub(keep);
    let Some(id) = input.prior_chat.iter().take(split).next_back() else {
        return false;
    };
    if stage.dropped_prior.iter().any(|d| d == &id.record_id) {
        return false;
    }
    stage.dropped_prior.push(id.record_id.clone());
    stage.prior_keep = keep;
    true
}

pub(crate) fn drop_oldest_memory(input: &BuildInput, stage: &mut Stage) -> bool {
    let Some(oldest) = least_relevant(&input.memories, &stage.dropped_memory) else {
        return false;
    };
    stage.dropped_memory.push(oldest.id.clone());
    true
}

fn least_relevant<'a>(
    memories: &'a [MemoryRecord],
    dropped: &[String],
) -> Option<&'a MemoryRecord> {
    let mut ranked: Vec<&MemoryRecord> = memories
        .iter()
        .filter(|m| !dropped.iter().any(|id| id == &m.id))
        .collect();
    ranked.sort_by(|a, b| {
        a.relevance
            .cmp(&b.relevance)
            .then_with(|| b.candidate.cmp(&a.candidate))
            .then_with(|| b.id.cmp(&a.id))
    });
    ranked.first().copied()
}
