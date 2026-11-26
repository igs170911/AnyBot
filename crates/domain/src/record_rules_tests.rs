use crate::record_rules::{
    AcceptContext, AcceptOutcome, BotState, DedupFields, decide_accept, dedup_same,
};

fn fields(kind: &str, content: &str, task: Option<&str>, at: i64) -> DedupFields {
    DedupFields {
        kind: kind.to_owned(),
        content: serde_json::from_str(content).unwrap(),
        task_id: task.map(str::to_owned),
        created_at_ms: at,
    }
}

#[test]
fn dedup_same_compares_the_whole_fields() {
    let a = fields("message", "{\"text\":\"hi\"}", None, 10);
    let b = fields("message", "{\"text\":\"bye\"}", None, 10);
    assert!(!dedup_same(&a, &b), "different content must reject");
    assert!(dedup_same(
        &a,
        &fields("message", "{\"text\":\"hi\"}", None, 10)
    ));
    assert!(!dedup_same(
        &a,
        &fields("answer", "{\"text\":\"hi\"}", None, 10)
    ));
    assert!(!dedup_same(
        &a,
        &fields("message", "{\"text\":\"hi\"}", Some("t1"), 10)
    ));
    assert!(!dedup_same(
        &a,
        &fields("message", "{\"text\":\"hi\"}", None, 11)
    ));
    assert!(!dedup_same(
        &a,
        &fields("message", "{\"text\":\"hi\",\"x\":1}", None, 10)
    ));
}

#[test]
fn t_sync_id_2() {
    let a = fields("message", "{\"text\":\"hi\"}", None, 10);
    assert!(dedup_same(&a, &a), "same record resent is a duplicate");
}

#[test]
fn accept_matrix() {
    let ok = AcceptContext::account_stream(true, false);
    assert_eq!(decide_accept("message", &ok), AcceptOutcome::Accept);
    assert_eq!(decide_accept("zzz-unknown", &ok), AcceptOutcome::Accept);
    assert_eq!(decide_accept("bots_create", &ok), AcceptOutcome::Accept);

    let ctx = AcceptContext::bot_stream(true, false, true, BotState::Missing);
    assert_eq!(decide_accept("message", &ctx), AcceptOutcome::BotNotCreated);

    let ctx = AcceptContext::bot_stream(true, false, true, BotState::Deleted);
    assert_eq!(decide_accept("message", &ctx), AcceptOutcome::BotDeleted);

    let ctx = AcceptContext::bot_stream(false, false, true, BotState::Live);
    assert_eq!(decide_accept("message", &ctx), AcceptOutcome::NoPermission);

    let ctx = AcceptContext::bot_stream(true, false, false, BotState::Live);
    assert_eq!(decide_accept("message", &ctx), AcceptOutcome::StreamMissing);

    let ctx = AcceptContext::account_stream(false, false);
    assert_eq!(decide_accept("message", &ctx), AcceptOutcome::NoPermission);

    let ctx = AcceptContext::account_stream(true, true);
    assert_eq!(
        decide_accept("message", &ctx),
        AcceptOutcome::AccountDisabled
    );

    for kind in crate::record_rules::DEVICE_FORBIDDEN_KINDS {
        let ctx = AcceptContext::account_stream(true, false);
        assert_eq!(decide_accept(kind, &ctx), AcceptOutcome::ForbiddenKind);
    }
}
