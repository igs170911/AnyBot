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

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EstimateError {
    O200kInit(String),
    MissingTokenizerJson,
    BrokenTokenizerJson(String),
    TokenizerJsonBlocked,
}

impl std::fmt::Display for EstimateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::O200kInit(detail) => write!(f, "failed to initialize o200k tokenizer: {detail}"),
            Self::MissingTokenizerJson => f.write_str("tokenizer.json bytes are required"),
            Self::BrokenTokenizerJson(detail) => write!(f, "invalid tokenizer.json: {detail}"),
            Self::TokenizerJsonBlocked => {
                f.write_str("tokenizer.json runtime is blocked by cargo-deny advisory policy")
            }
        }
    }
}

impl std::error::Error for EstimateError {}

fn o200k_bpe() -> Result<&'static tiktoken_rs::CoreBPE, EstimateError> {
    static O200K: std::sync::LazyLock<Result<tiktoken_rs::CoreBPE, String>> =
        std::sync::LazyLock::new(|| tiktoken_rs::o200k_base().map_err(|e| e.to_string()));
    O200K
        .as_ref()
        .map_err(|err| EstimateError::O200kInit(err.clone()))
}

fn validate_tokenizer_json(raw: Option<&[u8]>) -> Result<(), EstimateError> {
    let Some(bytes) = raw.filter(|slice| !slice.is_empty()) else {
        return Err(EstimateError::MissingTokenizerJson);
    };
    let parsed: serde_json::Value = serde_json::from_slice(bytes)
        .map_err(|err| EstimateError::BrokenTokenizerJson(err.to_string()))?;
    let Some(obj) = parsed.as_object() else {
        return Err(EstimateError::BrokenTokenizerJson(
            "root value must be a JSON object".to_owned(),
        ));
    };
    if !obj.get("model").is_some_and(serde_json::Value::is_object) {
        return Err(EstimateError::BrokenTokenizerJson(
            "missing model object".to_owned(),
        ));
    }
    Err(EstimateError::TokenizerJsonBlocked)
}

pub fn estimate_tokens(
    kind: &crate::capabilities::TokenizerKind,
    text: &str,
    tokenizer_json: Option<&[u8]>,
) -> Result<u64, EstimateError> {
    match kind {
        crate::capabilities::TokenizerKind::Formula => Ok(formula_tokens(&EstimateInput {
            text,
            messages: 0,
            images: 0,
        })),
        crate::capabilities::TokenizerKind::O200k => {
            let bpe = o200k_bpe()?;
            let count = bpe.encode_ordinary(text).len();
            u64::try_from(count).map_err(|err| EstimateError::O200kInit(err.to_string()))
        }
        crate::capabilities::TokenizerKind::TokenizerJson => {
            validate_tokenizer_json(tokenizer_json)?;
            Err(EstimateError::TokenizerJsonBlocked)
        }
    }
}

pub fn estimate_input_tokens(
    kind: &crate::capabilities::TokenizerKind,
    input: &EstimateInput<'_>,
    tokenizer_json: Option<&[u8]>,
) -> Result<u64, EstimateError> {
    let text_tokens = estimate_tokens(kind, input.text, tokenizer_json)?;
    Ok(text_tokens
        .saturating_add(input.messages.saturating_mul(MESSAGE_OVERHEAD))
        .saturating_add(input.images.saturating_mul(IMAGE_TOKENS)))
}
