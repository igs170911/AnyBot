use crate::compile::{Assembled, Stage};
use crate::input::BuildInput;
use crate::numbers::K114_4_WATERMARK_MILLI;
use crate::output::{
    Block, BlockKind, Compiled, CompiledRequest, Manifest, ManifestBudget, ManifestEntry, NewEvent,
    manifest_model,
};

pub(crate) struct SentParts<'a> {
    input: &'a BuildInput,
    assembled: &'a Assembled,
    stage: &'a Stage,
    usable: u64,
    total_raw: u64,
    cal: u64,
}

pub(crate) fn sent(
    input: &BuildInput,
    assembled: &Assembled,
    stage: &Stage,
    usable: u64,
    total_raw: u64,
    cal: u64,
) -> Compiled {
    let parts = SentParts {
        input,
        assembled,
        stage,
        usable,
        total_raw,
        cal,
    };
    let blocks: Vec<Block> = parts
        .assembled
        .prefix
        .iter()
        .chain(parts.assembled.core.iter())
        .chain(parts.assembled.tail.iter())
        .cloned()
        .collect();
    let prefix_len = stable_prefix_len(&blocks);
    let request = CompiledRequest {
        system: parts.input.system_prompt.clone(),
        blocks,
        prefix_len,
    };
    let mut events = parts.stage.events.clone();
    let manifest = build_manifest(&parts, &request);
    events.push(NewEvent::ContextBuilt);
    Compiled::Sent {
        request,
        manifest: Box::new(manifest),
        events,
    }
}

pub(crate) fn stable_prefix_len(blocks: &[Block]) -> usize {
    blocks
        .iter()
        .take_while(|b| !matches!(b.kind, BlockKind::StateSummary | BlockKind::Memory))
        .count()
}

fn build_manifest(parts: &SentParts<'_>, request: &CompiledRequest) -> Manifest {
    let budget = ManifestBudget {
        usable: parts.usable,
        reserved_output: parts.input.model.reserved_output,
        estimate_raw: parts.total_raw,
        estimate_calibrated: parts.cal,
        margin_milli: anybot_models::margin_milli(parts.input.model.calibration.samples()),
        watermark_milli: K114_4_WATERMARK_MILLI,
    };
    Manifest {
        included: request
            .blocks
            .iter()
            .map(|b| ManifestEntry {
                kind: b.kind,
                form: b.form,
                source_ids: b.source_ids.clone(),
                estimate_raw: b.estimate_raw,
            })
            .collect(),
        excluded: parts.assembled.excluded.clone(),
        budget,
        model: manifest_model(&parts.input.model),
        actual_input_tokens: None,
        estimate_delta: None,
    }
}
