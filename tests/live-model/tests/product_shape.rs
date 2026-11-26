use anybot_foundation as _;
use anybot_models as _;
use live_model::product::{compare_fakes, missing_profile_hint};
use reqwest as _;

use serde_json as _;
use test_harness as _;
use tokio as _;
use toml as _;

#[tokio::test]
async fn product_client_shapes_match_across_vllm_fakes() {
    compare_fakes().await.unwrap();
    assert!(missing_profile_hint().contains("profile.toml"));
}
