use anybot_foundation as _;
use anybot_models as _;
use bytes as _;
use futures_util as _;
use proptest as _;
use reqwest as _;
use rustls as _;
use serde as _;
use serde_json as _;
use test_harness as _;
use tokio as _;
use url as _;

#[test]
fn complete_without_permit_does_not_compile() {
    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/compilefail/complete_without_permit.rs");
}
