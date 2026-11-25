use super::{GapRecord, numbers_stable, reason_similarity, same_gap};

#[test]
fn t_goal_brake_9_same_item_with_moving_numbers_is_not_same() {
    let prev = GapRecord::new("upload", "checked 7 of 10 files ok");
    let next = GapRecord::new("upload", "checked 6 of 10 files ok");
    assert!(reason_similarity(&prev.reason, &next.reason) >= 0.7);
    assert!(!numbers_stable(&prev.reason, &next.reason));
    assert!(!same_gap(&prev, &next));
}

#[test]
fn t_goal_brake_9_same_item_with_steady_reason_is_same() {
    let prev = GapRecord::new("upload", "checked 7 of 10 files ok");
    let next = GapRecord::new("upload", "checked 7 of 10 files okay");
    assert!(same_gap(&prev, &next));
}

#[test]
fn test_brake_different_items_are_never_same() {
    let prev = GapRecord::new("upload", "same reason text");
    let next = GapRecord::new("download", "same reason text");
    assert!(!same_gap(&prev, &next));
}
