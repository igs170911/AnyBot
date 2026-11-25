use bytes::Bytes;

use crate::scripted::ScriptStep;
use crate::transport::TransportFail;

pub(crate) fn chunk_pieces(step: &ScriptStep) -> Vec<Result<Bytes, TransportFail>> {
    let source = if step.chunks.is_empty() {
        vec![step.body.clone()]
    } else {
        step.chunks.clone()
    };
    let mut out = Vec::new();
    let mut seen = 0usize;
    for piece in source {
        let kept = kept_piece(&piece, step.cut_at, seen);
        seen = seen.saturating_add(piece.len());
        if !kept.is_empty() {
            out.push(Ok(Bytes::from(kept)));
        }
        if step.cut_at.is_some_and(|at| seen >= at) {
            out.push(Err(TransportFail::Dropped));
            return out;
        }
    }
    out
}

fn kept_piece(piece: &str, cut_at: Option<usize>, seen: usize) -> String {
    let Some(at) = cut_at else {
        return piece.to_owned();
    };
    if seen >= at {
        return String::new();
    }
    let mut end = at.saturating_sub(seen).min(piece.len());
    while end > 0 && !piece.is_char_boundary(end) {
        end = end.saturating_sub(1);
    }
    match piece.get(..end) {
        Some(text) => text.to_owned(),
        None => String::new(),
    }
}
