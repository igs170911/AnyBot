use anybot_context as _;
use anybot_domain as _;
use anybot_foundation as _;
use anybot_messages as _;
use anybot_models as _;
use anybot_protocol as _;
use anybot_runner as _;
use anybot_store_client as _;
use anybot_tools as _;
use anybot_trust as _;
use proptest as _;
use serde_json as _;
use sha2 as _;
use test_harness as _;
use thiserror as _;

use runner_test::turn::{Device, TurnWorld, WorldSetup};
use runner_test::turn_helpers::{chat_requests, kind_names, task_of};
use test_harness::model_replies_text;

#[tokio::test]
async fn t_claims_check_6() -> Result<(), String> {
    let world = TurnWorld::start(&WorldSetup::default()).await?;
    world.script(vec![model_replies_text("\u{6211}\u{9084}\u{6c92}\u{5b8c}\u{6210}\u{ff0c}\u{5b8c}\u{6210}\u{5f8c}\u{6703}\u{50b3}\u{7d66}\u{4f60}\u{3002}", false)]);
    let outcome = world
        .send(Device::Phone, "report back")
        .await
        .map_err(|e| format!("{e:?}"))?;
    let names = kind_names(&world, &task_of(&outcome));
    assert!(!names.contains(&"claim_checked".to_owned()));
    assert!(!names.contains(&"wake".to_owned()));
    assert_eq!(chat_requests(&world.model).len(), 1);
    Ok(())
}
