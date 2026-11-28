use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::PushError;

pub const DEFAULT_TITLE: &str = "AnyBot";
pub const TITLE_MAX: usize = 80;
pub const BODY_MAX: usize = 200;
pub const QUESTION_MAX: usize = 100;
pub const TAG_MAX: usize = 40;
pub const TOPIC_MAX: usize = 32;
pub const BADGE_MAX: u32 = 999;
pub const PAYLOAD_MAX: usize = 3993;

const QUESTION_PARAM: &str = "question";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PushMessage {
    pub title: String,
    pub body_code: String,
    pub body_params: BTreeMap<String, String>,
    pub tag: String,
    pub badge: u16,
}

fn truncated(text: &str, limit: usize) -> String {
    text.chars().take(limit).collect()
}

fn clean_tag(tag: &str) -> String {
    tag.chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
        .take(TAG_MAX)
        .collect()
}

fn param_limit(name: &str) -> usize {
    if name == QUESTION_PARAM {
        QUESTION_MAX
    } else {
        BODY_MAX
    }
}

fn clamp_badge(badge: u32) -> u16 {
    u16::try_from(badge.min(BADGE_MAX)).unwrap_or(u16::MAX)
}

impl PushMessage {
    #[must_use]
    pub fn compose(
        title: &str,
        body_code: &str,
        body_params: &[(&str, &str)],
        tag: &str,
        badge: u32,
    ) -> PushMessage {
        let title = if title.is_empty() {
            DEFAULT_TITLE.to_owned()
        } else {
            truncated(title, TITLE_MAX)
        };
        let body_params = body_params
            .iter()
            .map(|(name, value)| ((*name).to_owned(), truncated(value, param_limit(name))))
            .collect();
        PushMessage {
            title,
            body_code: body_code.to_owned(),
            body_params,
            tag: clean_tag(tag),
            badge: clamp_badge(badge),
        }
    }

    #[must_use]
    pub fn topic(&self) -> Option<String> {
        let topic = truncated(&self.tag, TOPIC_MAX);
        if topic.is_empty() { None } else { Some(topic) }
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, PushError> {
        let bytes =
            serde_json::to_vec(self).map_err(|source| PushError::Serialize(source.to_string()))?;
        if bytes.len() > PAYLOAD_MAX {
            return Err(PushError::PayloadTooLarge);
        }
        Ok(bytes)
    }
}
