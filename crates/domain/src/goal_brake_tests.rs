use super::{GapRecord, numbers_stable, reason_similarity, same_gap};

#[test]
fn t_goal_brake_9() {
    let prev = GapRecord::new("upload", "checked 7 of 10 files ok");
    let moving = GapRecord::new("upload", "checked 6 of 10 files ok");
    let steady = GapRecord::new("upload", "checked 7 of 10 files okay");
    assert!(reason_similarity(&prev.reason, &moving.reason) >= 0.7);
    assert!(!numbers_stable(&prev.reason, &moving.reason));
    assert!(!same_gap(&prev, &moving));
    assert!(same_gap(&prev, &steady));
}

#[test]
fn test_brake_different_items_are_never_same() {
    let prev = GapRecord::new("upload", "same reason text");
    let next = GapRecord::new("download", "same reason text");
    assert!(!same_gap(&prev, &next));
}
