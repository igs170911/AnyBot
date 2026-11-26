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
fn t_install_n25_5_verdict() {
    let verdict = judge_client_version(Some("0.2.0"), &table(), BASE + 220 * DAY);
    assert_eq!(verdict, VersionVerdict::TooOld);
}

#[test]
fn t_install_n25_5_accept_inside_window() {
    let verdict = judge_client_version(Some("0.2.0"), &table(), BASE + 200 * DAY);
    assert_eq!(verdict, VersionVerdict::Accept);
}

#[test]
fn t_install_n25_2_replaced_version_accept() {
    let verdict = judge_client_version(Some("0.3.0"), &table(), BASE + 220 * DAY);
    assert_eq!(verdict, VersionVerdict::Accept);
}

#[test]
fn t_install_n25_1_unreadable_header() {
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
