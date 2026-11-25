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

fn kept_piece(piece: &str, cut_at: Option<usize>, seen: usize) -> Vec<u8> {
    let bytes = piece.as_bytes();
    let Some(at) = cut_at else {
        return bytes.to_vec();
    };
    if seen >= at {
        return Vec::new();
    }
    let end = at.saturating_sub(seen).min(bytes.len());
    bytes.get(..end).unwrap_or(bytes).to_vec()
}
