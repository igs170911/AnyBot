use anybot_models::{Calibration, EstimateInput, apply_calibration, formula_tokens};

use crate::numbers::{K114_4_WATERMARK_MILLI, milli_of};
use crate::output::Block;

#[must_use]
pub fn text_tokens(text: &str) -> u64 {
    formula_tokens(&EstimateInput {
        text,
        messages: 1,
        images: 0,
    })
}

#[must_use]
pub fn blocks_raw(blocks: &[Block]) -> u64 {
    blocks.iter().map(|b| b.estimate_raw).sum()
}

#[must_use]
pub fn calibrated(total_raw: u64, calibration: &Calibration) -> u64 {
    apply_calibration(total_raw, calibration)
}

#[must_use]
pub fn under_watermark(value: u64, usable: u64) -> bool {
    value <= milli_of(usable, K114_4_WATERMARK_MILLI)
}
