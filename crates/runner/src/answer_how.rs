use anybot_protocol::{AnswerOrigin, JudgementSource};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnswerHow {
    pub origin: AnswerOrigin,
    pub source: JudgementSource,
    pub smart_wait_seconds: Option<u32>,
}

impl AnswerHow {
    #[must_use]
    pub fn hand() -> Self {
        AnswerHow {
            origin: AnswerOrigin::Hand,
            source: JudgementSource::You,
            smart_wait_seconds: None,
        }
    }

    #[must_use]
    pub fn stop() -> Self {
        AnswerHow {
            origin: AnswerOrigin::Stop,
            source: JudgementSource::You,
            smart_wait_seconds: None,
        }
    }

    #[must_use]
    pub fn yolo() -> Self {
        AnswerHow {
            origin: AnswerOrigin::Auto,
            source: JudgementSource::AlwaysApprove,
            smart_wait_seconds: None,
        }
    }

    #[must_use]
    pub fn smart(seconds: u32) -> Self {
        AnswerHow {
            origin: AnswerOrigin::Auto,
            source: JudgementSource::RuleTimeout,
            smart_wait_seconds: Some(seconds),
        }
    }
}
