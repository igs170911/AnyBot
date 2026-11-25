use anybot_foundation as _;
use anybot_models as _;
use live_model::side::fake_side;
use reqwest as _;

use serde_json as _;
use test_harness as _;
use tokio as _;
use toml as _;

#[tokio::test]
async fn live_fake_side_completes_all_seven_requests() {
    let outcome = fake_side("local").await;
    assert_eq!(outcome.failed, None);
    assert_eq!(outcome.side.probe.len(), 3);
    assert_eq!(outcome.side.product.len(), 4);
}

#[tokio::test]
async fn live_fake_side_probe_shapes_match_scripts() {
    let outcome = fake_side("local").await;
    assert_eq!(outcome.failed, None);
    let normal = outcome.side.probe.first().expect("probe normal");
    assert!(
        normal.contains(&"finish_reason=stop".to_owned()),
        "{normal:?}"
    );
    let stream = outcome.side.probe.get(1).expect("probe stream");
    assert!(stream.contains(&"end=[DONE]".to_owned()), "{stream:?}");
    let tool = outcome.side.probe.get(2).expect("probe tool");
    assert!(tool.contains(&"tool_calls:count=1".to_owned()), "{tool:?}");
}

#[tokio::test]
async fn live_fake_side_product_shapes_match_scripts_in_order() {
    let outcome = fake_side("local").await;
    assert_eq!(outcome.failed, None);
    let product = &outcome.side.product;
    assert_eq!(product.first().expect("product normal").stop, "finished");
    assert_eq!(product.get(1).expect("product stream").stop, "finished");
    assert_eq!(product.get(2).expect("product tool").stop, "needs-tool");
    assert_eq!(product.get(2).expect("product tool").tool_calls, 1);
    assert_eq!(product.get(3).expect("product thinking").stop, "finished");
    assert!(product.get(3).expect("product thinking").has_thinking);
}
