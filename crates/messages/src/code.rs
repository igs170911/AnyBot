use crate::catalog::MessageCodeLike;

#[derive(Clone, Debug, PartialEq)]
pub struct TestCode(String);

impl TestCode {
    #[must_use]
    pub fn new(text: &str) -> Self {
        TestCode(text.to_owned())
    }
}

impl MessageCodeLike for TestCode {
    fn as_str(&self) -> &str {
        &self.0
    }
}
