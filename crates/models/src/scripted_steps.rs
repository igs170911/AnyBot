use std::sync::Arc;

use crate::scripted::{ScriptStep, script_chunks, script_http, script_step};

pub(crate) fn script_cut(status: u16, body: &str, at: usize) -> ScriptStep {
    let mut step = script_step(status, body.to_owned(), false);
    step.chunks = byte_slices(body);
    step.cut_at = Some(at);
    step
}

pub(crate) fn byte_slices(body: &str) -> Vec<String> {
    let bytes = body.as_bytes();
    if bytes.is_empty() {
        return vec![String::new()];
    }
    let width = bytes.len().saturating_div(4).max(1);
    let mut out = Vec::new();
    let mut at = 0usize;
    while at < bytes.len() {
        let end = at.saturating_add(width).min(bytes.len());
        let piece = bytes.get(at..end).unwrap_or(&[]);
        out.push(encode_slice(piece));
        at = end;
    }
    out
}

pub(crate) fn decoded_piece(piece: &str) -> Vec<u8> {
    let Some(rest) = piece.strip_prefix("hex:") else {
        return piece.as_bytes().to_vec();
    };
    match decode_hex(rest) {
        Ok(bytes) => bytes,
        Err(()) => piece.as_bytes().to_vec(),
    }
}

fn encode_slice(piece: &[u8]) -> String {
    if std::str::from_utf8(piece).is_ok() {
        return String::from_utf8_lossy(piece).into_owned();
    }
    format!("hex:{}", hex_of(piece))
}

fn hex_of(piece: &[u8]) -> String {
    const HEX: &[u8] = b"0123456789abcdef";
    let mut out = String::new();
    for byte in piece {
        let high = usize::from(byte >> 4);
        let low = usize::from(byte & 0x0f);
        out.push(HEX.get(high).copied().unwrap_or(b'0') as char);
        out.push(HEX.get(low).copied().unwrap_or(b'0') as char);
    }
    out
}

fn decode_hex(text: &str) -> Result<Vec<u8>, ()> {
    if !text.len().is_multiple_of(2) {
        return Err(());
    }
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut index = 0usize;
    while index < bytes.len() {
        let high = bytes.get(index).copied().ok_or(())?;
        let low = bytes.get(index.saturating_add(1)).copied().ok_or(())?;
        let value = nibble(high)?.checked_shl(4).ok_or(())? | nibble(low)?;
        out.push(value);
        index = index.saturating_add(2);
    }
    Ok(out)
}

fn nibble(byte: u8) -> Result<u8, ()> {
    match byte {
        b'0'..=b'9' => byte.checked_sub(b'0').ok_or(()),
        b'a'..=b'f' => byte
            .checked_sub(b'a')
            .and_then(|gap| gap.checked_add(10))
            .ok_or(()),
        _ => Err(()),
    }
}

pub(crate) fn raw_cut(status: u16, body: &str, at: usize) -> ScriptStep {
    let mut step = script_step(status, body.to_owned(), false);
    step.chunks = char_slices(body);
    step.cut_at = Some(at);
    step
}

fn char_slices(body: &str) -> Vec<String> {
    if body.is_empty() {
        return vec![String::new()];
    }
    let chars: Vec<char> = body.chars().collect();
    let width = chars.len().saturating_div(4).max(1);
    let mut out = Vec::new();
    let mut at = 0usize;
    while at < chars.len() {
        let end = at.saturating_add(width).min(chars.len());
        let piece: String = chars.get(at..end).unwrap_or(&[]).iter().collect();
        out.push(piece);
        at = end;
    }
    out
}

pub(crate) fn held(mut step: ScriptStep, hold: Arc<tokio::sync::Notify>) -> ScriptStep {
    step.hold = Some(hold);
    step
}

pub(crate) fn stall_step() -> ScriptStep {
    script_chunks(
        200,
        &[
            "data: {\"choices\":[{\"delta\":{\"content\":\"kept\"}}]}\n\n".to_owned(),
            "STALL".to_owned(),
        ],
    )
}

pub(crate) fn blank_step() -> ScriptStep {
    script_http(
        200,
        "{\"choices\":[{\"message\":{\"role\":\"assistant\",\"content\":\"\"},\"finish_reason\":\"stop\"}]}",
    )
}

pub(crate) fn reject_step() -> ScriptStep {
    script_http(
        400,
        "{\"error\":{\"message\":\"unknown parameter thinking\"}}",
    )
}

pub(crate) fn first_token_step() -> ScriptStep {
    let mut step = script_step(200, String::new(), false);
    step.cut_at = Some(0);
    step
}
