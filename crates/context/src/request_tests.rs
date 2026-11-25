use crate::compile::compile;
use crate::compile_tests_support as sup;
use crate::output::{BlockKind, Compiled};
use anybot_models::{Role, ThinkingMode, ToolSpec};

#[test]
fn turn_request_carries_plan_fields() {
    let mut base = sup::base_input();
    base.user_turns = vec![sup::record("u1", "hello there")];
    let compiled = compile(&base).unwrap();
    if let Compiled::Sent { request, .. } = compiled {
        let tools: Vec<ToolSpec> = vec![ToolSpec {
            name: "read_result".to_owned(),
            description: "read a record".to_owned(),
            parameters: serde_json::Value::Null,
        }];
        let turn = request.to_turn_request(tools.clone(), ThinkingMode::Auto, Some(1_024));
        assert_eq!(turn.system, request.system);
        assert_eq!(turn.messages.len(), request.blocks.len());
        assert_eq!(turn.tools, tools);
        assert_eq!(turn.max_output_tokens, Some(1_024));
        assert_eq!(turn.thinking, ThinkingMode::Auto);
        assert!(turn.cache_prefix);
        let user = request.blocks.iter().any(|b| b.kind == BlockKind::UserText);
        assert!(user);
        assert!(!turn.messages.is_empty());
    } else {
        panic!("expected sent");
    }
}

#[test]
fn tool_blocks_map_to_tool_role() {
    let base = sup::base_input();
    let compiled = compile(&base).unwrap();
    if let Compiled::Sent { request, .. } = compiled {
        let pairs: Vec<(BlockKind, Role)> = request
            .blocks
            .iter()
            .map(|b| (b.kind, b.kind.role()))
            .collect();
        for (kind, role) in pairs {
            if matches!(
                kind,
                BlockKind::ToolIntent
                    | BlockKind::ToolResultFull
                    | BlockKind::ToolResultMasked
                    | BlockKind::ToolResultDuplicate
            ) {
                assert_eq!(role, Role::Tool, "tool block must map to tool role");
            }
        }
        let turn = request.to_turn_request(Vec::new(), ThinkingMode::Auto, None);
        for (block, message) in request.blocks.iter().zip(turn.messages.iter()) {
            assert_eq!(message.role, block.kind.role());
            assert_eq!(message.text, block.text);
        }
    } else {
        panic!("expected sent");
    }
}
