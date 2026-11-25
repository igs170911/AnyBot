use serde_json::Value;

use crate::types::Usage;

#[must_use]
pub(crate) fn usage_of(value: &Value) -> Option<Usage> {
    let usage = value.get("usage")?;
    if usage.is_null() {
        return None;
    }
    let prompt = crate::jsonutil::u64_at(usage, "prompt_tokens")?;
    let output = crate::jsonutil::u64_at(usage, "completion_tokens").unwrap_or(0);
    let cache = usage
        .get("prompt_tokens_details")
        .and_then(|d| crate::jsonutil::u64_at(d, "cached_tokens"))
        .unwrap_or(0);
    let thinking = usage
        .get("completion_tokens_details")
        .and_then(|d| crate::jsonutil::u64_at(d, "reasoning_tokens"))
        .unwrap_or(0);
    let fresh = prompt.saturating_sub(cache);
    Some(Usage {
        input_tokens: fresh,
        cache_read_tokens: cache,
        cache_write_tokens: 0,
        output_tokens: output,
        thinking_tokens: thinking,
    })
}
