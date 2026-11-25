use serde_json::Value;

use crate::types::Usage;

#[must_use]
pub(crate) fn usage_of(value: Option<&Value>) -> Option<Usage> {
    let usage = value?;
    if usage.is_null() {
        return None;
    }
    Some(Usage {
        input_tokens: crate::jsonutil::u64_at(usage, "input_tokens").unwrap_or(0),
        cache_read_tokens: crate::jsonutil::u64_at(usage, "cache_read_input_tokens").unwrap_or(0),
        cache_write_tokens: crate::jsonutil::u64_at(usage, "cache_creation_input_tokens")
            .unwrap_or(0),
        output_tokens: crate::jsonutil::u64_at(usage, "output_tokens").unwrap_or(0),
        thinking_tokens: 0,
    })
}

pub(crate) fn merge_usage(left: &mut Usage, right: &Usage) {
    if right.input_tokens > 0 {
        left.input_tokens = right.input_tokens;
    }
    if right.cache_read_tokens > 0 {
        left.cache_read_tokens = right.cache_read_tokens;
    }
    if right.cache_write_tokens > 0 {
        left.cache_write_tokens = right.cache_write_tokens;
    }
    if right.output_tokens > 0 {
        left.output_tokens = right.output_tokens;
    }
}

impl Usage {
    pub(crate) fn default_zero() -> Self {
        Self {
            input_tokens: 0,
            cache_read_tokens: 0,
            cache_write_tokens: 0,
            output_tokens: 0,
            thinking_tokens: 0,
        }
    }
}
