use crate::version_release::{
    ReleaseRow, ReleaseTable, VersionVerdict, judge_client_version, six_months_after,
};

const BASE: i64 = 1_700_000_000_000;
const DAY: i64 = 86_400_000;

fn table() -> ReleaseTable {
    ReleaseTable {
        rows: vec![
            ReleaseRow {
                version: "0.2.0".to_string(),
                at_ms: BASE,
            },
            ReleaseRow {
                version: "0.3.0".to_string(),
                at_ms: BASE + 30 * DAY,
            },
        ],
    }
}

#[test]
fn verdict_is_too_old_past_the_six_month_window() {
    let verdict = judge_client_version(Some("0.2.0"), &table(), BASE + 220 * DAY);
    assert_eq!(verdict, VersionVerdict::TooOld);
}

#[test]
fn verdict_accepts_inside_the_six_month_window() {
    let verdict = judge_client_version(Some("0.2.0"), &table(), BASE + 200 * DAY);
    assert_eq!(verdict, VersionVerdict::Accept);
}

#[test]
fn verdict_accepts_the_replacing_version_forever() {
    let verdict = judge_client_version(Some("0.3.0"), &table(), BASE + 220 * DAY);
    assert_eq!(verdict, VersionVerdict::Accept);
}

#[test]
fn verdict_is_unreadable_for_junk_and_missing_headers() {
    let verdict = judge_client_version(Some("banana"), &table(), BASE + 220 * DAY);
    assert_eq!(verdict, VersionVerdict::Unreadable);
    let verdict = judge_client_version(None, &table(), BASE + 220 * DAY);
    assert_eq!(verdict, VersionVerdict::Unreadable);
}

#[test]
fn empty_table_always_accepts() {
    let verdict = judge_client_version(Some("0.1.0"), &ReleaseTable::empty(), BASE);
    assert_eq!(verdict, VersionVerdict::Accept);
}

#[test]
fn six_months_window_is_over_180_days() {
    let edge = six_months_after(BASE + 30 * DAY);
    assert!(edge - (BASE + 30 * DAY) > 180 * DAY);
    assert!(edge - (BASE + 30 * DAY) < 187 * DAY);
}
