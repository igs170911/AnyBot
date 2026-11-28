use std::collections::{BTreeMap, BTreeSet};

use anybot_context::read::{RawStore as CtxRawStore, ReadOutcome, ReadQuery, read_result};
use anybot_foundation::ContentDigest;
use anybot_protocol::{
    EvidencePayload, KnownTaskEventKind, TaskEventKind, ToolEffect, ToolRequest,
};
use anybot_tools::read_result::{
    RAW_MAX_BYTES, find_hits, read_result_cut_code, read_result_hit_code, read_result_missing_code,
    read_result_not_found_code, read_result_window_code,
};
use anybot_tools::{ReadResultArgs, result_id};
use anybot_trust::OutcomeText;

use crate::error::TurnError;
use crate::record::StoredBody;
use crate::runner::{TurnCtx, TurnRunner};
use crate::view_text::render_text;

pub(crate) async fn run_read_result(
    runner: &TurnRunner,
    ctx: &TurnCtx,
    request: &ToolRequest,
    args: &ReadResultArgs,
) -> Result<(), TurnError> {
    let summary_json = crate::action::args_json(&request.arguments)?;
    runner
        .append_task_event(
            &ctx.task,
            KnownTaskEventKind::ToolIntent {
                call_id: request.call_id,
                name: request.tool.clone(),
                args_summary: summary_json,
                level: ToolEffect::ReadOnly,
            },
        )
        .await?;
    runner
        .append_task_event(
            &ctx.task,
            KnownTaskEventKind::ToolStarted {
                call_id: request.call_id,
            },
        )
        .await?;
    let summary = fetch_and_format(runner, ctx, args).await?;
    runner
        .append_task_event(
            &ctx.task,
            KnownTaskEventKind::ToolCompleted {
                call_id: request.call_id,
                summary,
                raw_id: None,
            },
        )
        .await
        .map(|_| ())
}

async fn fetch_and_format(
    runner: &TurnRunner,
    ctx: &TurnCtx,
    args: &ReadResultArgs,
) -> Result<String, TurnError> {
    let Some(digest) = resolve_digest(runner, ctx, &args.id).await? else {
        return missing_text(runner);
    };
    let Some(blob) = runner.raw.get(digest).await.map_err(TurnError::Raw)? else {
        return missing_text(runner);
    };
    let text = String::from_utf8_lossy(&blob.bytes).into_owned();
    let truncated = blob.original_len > u64::try_from(blob.bytes.len()).unwrap_or(u64::MAX);
    format_blob(runner, args, &text, truncated)
}

fn format_blob(
    runner: &TurnRunner,
    args: &ReadResultArgs,
    text: &str,
    truncated: bool,
) -> Result<String, TurnError> {
    let mut records = BTreeMap::new();
    records.insert(args.id.clone(), text.to_owned());
    let store = CtxRawStore::new(records, BTreeSet::new());
    if let Some(needle) = &args.find {
        return format_find(runner, &store, &args.id, text, needle, truncated);
    }
    let query = ReadQuery::Window {
        from: args.from,
        chars: args.chars,
    };
    match read_result(&store, &args.id, &query) {
        ReadOutcome::Window {
            from,
            to,
            total,
            text: slice,
        } => format_window(runner, from, to, total, &slice, truncated),
        ReadOutcome::Text { .. }
        | ReadOutcome::Found { .. }
        | ReadOutcome::NotFound
        | ReadOutcome::SourceDeleted => missing_text(runner),
    }
}

fn format_window(
    runner: &TurnRunner,
    from: usize,
    to: usize,
    total: usize,
    slice: &str,
    truncated: bool,
) -> Result<String, TurnError> {
    let header = render_text(
        runner.lang,
        &OutcomeText::of(
            read_result_window_code(),
            vec![
                ("from".to_owned(), from.to_string()),
                ("to".to_owned(), to.to_string()),
                ("total".to_owned(), total.to_string()),
            ],
        ),
    )?;
    let mut lines = vec![header];
    if truncated {
        lines.push(cut_text(runner)?);
    }
    if !slice.is_empty() {
        lines.push(slice.to_owned());
    }
    Ok(lines.join("\n"))
}

fn format_find(
    runner: &TurnRunner,
    store: &CtxRawStore,
    id: &str,
    text: &str,
    needle: &str,
    truncated: bool,
) -> Result<String, TurnError> {
    let first = read_result(
        store,
        id,
        &ReadQuery::Find {
            needle: needle.to_owned(),
        },
    );
    if matches!(first, ReadOutcome::NotFound | ReadOutcome::SourceDeleted) {
        return render_text(
            runner.lang,
            &OutcomeText::of(read_result_not_found_code(), Vec::new()),
        );
    }
    let hits = find_hits(text, needle);
    if hits.is_empty() {
        return render_text(
            runner.lang,
            &OutcomeText::of(read_result_not_found_code(), Vec::new()),
        );
    }
    let mut lines = Vec::new();
    for hit in hits {
        lines.push(render_text(
            runner.lang,
            &OutcomeText::of(
                read_result_hit_code(),
                vec![
                    ("at".to_owned(), hit.at.to_string()),
                    ("context".to_owned(), hit.context),
                ],
            ),
        )?);
    }
    if truncated {
        lines.push(cut_text(runner)?);
    }
    Ok(lines.join("\n"))
}

fn cut_text(runner: &TurnRunner) -> Result<String, TurnError> {
    let mb = RAW_MAX_BYTES / 1_048_576;
    render_text(
        runner.lang,
        &OutcomeText::of(
            read_result_cut_code(),
            vec![("mb".to_owned(), mb.to_string())],
        ),
    )
}

fn missing_text(runner: &TurnRunner) -> Result<String, TurnError> {
    render_text(
        runner.lang,
        &OutcomeText::of(read_result_missing_code(), Vec::new()),
    )
}

async fn resolve_digest(
    runner: &TurnRunner,
    ctx: &TurnCtx,
    id: &str,
) -> Result<Option<ContentDigest>, TurnError> {
    let task_events = runner
        .record
        .task_events(ctx.task)
        .await
        .map_err(TurnError::Record)?;
    if let Some(found) = task_events.iter().find_map(|ev| match &ev.kind {
        TaskEventKind::Known(kind) => digest_from_kind(kind, id),
        TaskEventKind::Raw(_) => None,
    }) {
        return Ok(Some(found));
    }
    let stored = runner
        .record
        .conversation(runner.bot.id)
        .await
        .map_err(TurnError::Record)?;
    for row in &stored {
        if let StoredBody::Task { event, .. } = &row.body
            && let TaskEventKind::Known(kind) = &event.kind
            && let Some(found) = digest_from_kind(kind, id)
        {
            return Ok(Some(found));
        }
    }
    Ok(ContentDigest::parse(id).ok())
}

fn digest_from_kind(kind: &KnownTaskEventKind, id: &str) -> Option<ContentDigest> {
    if let KnownTaskEventKind::ToolCompleted {
        raw_id: Some(hex), ..
    } = kind
    {
        let digest = ContentDigest::parse(hex).ok()?;
        return (result_id(&digest) == id).then_some(digest);
    }
    if let KnownTaskEventKind::EvidenceAdded { record } = kind
        && let EvidencePayload::Executed { full_digest, .. } = &record.payload
    {
        return (result_id(full_digest) == id).then(|| full_digest.clone());
    }
    None
}
