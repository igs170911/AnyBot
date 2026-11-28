use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum QuestionKind {
    Single,
    Multi,
    Text,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
pub struct QuestionItem {
    pub title: String,
    pub kind: QuestionKind,
    pub options: Vec<String>,
    pub prefill: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
#[serde(rename_all = "snake_case", tag = "kind", content = "values")]
pub enum QuestionAnswer {
    Answers(Vec<String>),
    Skipped,
    Unanswered,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn question_types_round_trip() {
        let item = QuestionItem {
            title: "Branch?".to_owned(),
            kind: QuestionKind::Multi,
            options: vec!["main".to_owned(), "dev".to_owned()],
            prefill: Some("main".to_owned()),
        };
        let json = serde_json::to_string(&item).unwrap();
        let back: QuestionItem = serde_json::from_str(&json).unwrap();
        assert_eq!(back, item);

        for ans in [
            QuestionAnswer::Answers(vec!["a".to_owned(), "b".to_owned()]),
            QuestionAnswer::Skipped,
            QuestionAnswer::Unanswered,
        ] {
            let text = serde_json::to_string(&ans).unwrap();
            let got: QuestionAnswer = serde_json::from_str(&text).unwrap();
            assert_eq!(got, ans);
        }
    }
}
