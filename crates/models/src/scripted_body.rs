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
        if piece == "STALL" {
            out.push(Err(TransportFail::Partial(crate::transport::PartialBody {
                status: 200,
                bytes: b"data: {\"choices\":[{\"delta\":{\"content\":\"kept\"}}]}\n\n".to_vec(),
                stalled: true,
            })));
            return out;
        }
        let raw = crate::scripted_steps::decoded_piece(&piece);
        let kept = kept_bytes(&raw, step.cut_at, seen);
        seen = seen.saturating_add(raw.len());
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

fn kept_bytes(bytes: &[u8], cut_at: Option<usize>, seen: usize) -> Vec<u8> {
    let Some(at) = cut_at else {
        return bytes.to_vec();
    };
    if seen >= at {
        return Vec::new();
    }
    let end = at.saturating_sub(seen).min(bytes.len());
    bytes.get(..end).unwrap_or(bytes).to_vec()
}
