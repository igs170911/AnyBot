pub const MARGIN_COLD_MILLI: u64 = 1_150;
pub const MARGIN_WARM_MILLI: u64 = 1_100;
pub const COLD_SAMPLES: usize = 5;
pub const WINDOW: usize = 20;
pub const IMAGE_TOKENS: u64 = 1_600;
pub const MESSAGE_OVERHEAD: u64 = 4;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EstimateInput<'a> {
    pub text: &'a str,
    pub messages: u64,
    pub images: u64,
}

#[must_use]
pub fn formula_tokens(input: &EstimateInput<'_>) -> u64 {
    let mut ascii = 0_u64;
    let mut other = 0_u64;
    for ch in input.text.chars() {
        if ch.is_ascii() {
            ascii = ascii.saturating_add(1);
        } else {
            other = other.saturating_add(1);
        }
    }
    let text = ascii.div_ceil(4).saturating_add(other);
    text.saturating_add(input.messages.saturating_mul(MESSAGE_OVERHEAD))
        .saturating_add(input.images.saturating_mul(IMAGE_TOKENS))
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Calibration {
    pub ratios_milli: Vec<u64>,
}

impl Calibration {
    #[must_use]
    pub fn new() -> Self {
        Self {
            ratios_milli: Vec::new(),
        }
    }

    pub fn record(&mut self, estimated: u64, actual: u64) {
        if estimated == 0 {
            return;
        }
        let ratio = actual
            .saturating_mul(1_000)
            .checked_div(estimated)
            .unwrap_or(1_000);
        self.ratios_milli.push(ratio);
        let len = self.ratios_milli.len();
        if len > WINDOW {
            let extra = len.saturating_sub(WINDOW);
            self.ratios_milli.drain(0..extra);
        }
    }

    #[must_use]
    pub fn samples(&self) -> usize {
        self.ratios_milli.len()
    }

    #[must_use]
    pub fn mean_milli(&self) -> u64 {
        if self.ratios_milli.is_empty() {
            return 1_000;
        }
        let sum: u64 = self
            .ratios_milli
            .iter()
            .fold(0, |acc, item| acc.saturating_add(*item));
        let count = u64::try_from(self.ratios_milli.len()).unwrap_or(1);
        sum.checked_div(count).unwrap_or(1_000)
    }
}

impl Default for Calibration {
    fn default() -> Self {
        Self::new()
    }
}

#[must_use]
pub fn margin_milli(samples: usize) -> u64 {
    if samples < COLD_SAMPLES {
        MARGIN_COLD_MILLI
    } else {
        MARGIN_WARM_MILLI
    }
}

#[must_use]
pub fn apply_calibration(raw: u64, calibration: &Calibration) -> u64 {
    let scaled = raw
        .saturating_mul(calibration.mean_milli())
        .saturating_add(999)
        / 1_000;
    scaled
        .saturating_mul(margin_milli(calibration.samples()))
        .saturating_add(999)
        / 1_000
}

#[must_use]
pub fn fits(estimated_with_margin: u64, usable: u64) -> bool {
    estimated_with_margin <= usable
}
