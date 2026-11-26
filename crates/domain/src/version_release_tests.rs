use crate::version_release::{ReleaseRow, ReleaseTable, VersionVerdict, judge_client_version};

fn table(pairs: &[(&str, i64)]) -> ReleaseTable {
    ReleaseTable {
        rows: pairs
            .iter()
            .map(|(version, at_ms)| ReleaseRow {
                version: (*version).to_owned(),
                at_ms: *at_ms,
            })
            .collect(),
    }
}

#[test]
fn t_install_n25_1() {
    let table = table(&[("0.2.0", 1_000_000_000_000)]);
    let late = 1_000_000_000_000 + 210 * 86_400_000;
    assert_eq!(
        judge_client_version(Some("0.1.0"), &table, late),
        VersionVerdict::TooOld
    );
}

#[test]
fn t_install_n25_2() {
    let table = table(&[("0.2.0", 1_000_000_000_000)]);
    let soon = 1_000_000_000_000 + 150 * 86_400_000;
    assert_eq!(
        judge_client_version(Some("0.1.0"), &table, soon),
        VersionVerdict::Accept
    );
}

#[test]
fn t_install_n25_5() {
    let table = table(&[("0.2.0", 1_000_000_000_000)]);
    assert_eq!(
        judge_client_version(None, &table, 1_000_000_000_000),
        VersionVerdict::Unreadable
    );
    assert_eq!(
        judge_client_version(Some("0.1"), &table, 1_000_000_000_000),
        VersionVerdict::Unreadable
    );
    assert_eq!(
        judge_client_version(Some("not-a-version"), &table, 1_000_000_000_000),
        VersionVerdict::Unreadable
    );
}

#[test]
fn t_install_n25_6() {
    let table = table(&[("0.2.0", 1_000_000_000_000)]);
    let one_month = 1_000_000_000_000 + 30 * 86_400_000;
    assert_eq!(
        judge_client_version(Some("0.3.0"), &table, one_month),
        VersionVerdict::Accept
    );
    assert_eq!(
        judge_client_version(Some("0.1.0"), &table, one_month),
        VersionVerdict::Accept
    );
}

#[test]
fn version_release_empty_table_accepts_well_formed_versions() {
    assert_eq!(
        judge_client_version(Some("9.9.9"), &ReleaseTable::empty(), 1),
        VersionVerdict::Accept
    );
    assert_eq!(
        judge_client_version(Some("1.2.3-rc.1"), &ReleaseTable::empty(), 1),
        VersionVerdict::Accept
    );
}

#[test]
fn version_release_equal_latest_is_accepted() {
    let table = table(&[("0.2.0", 1_000_000_000_000)]);
    assert_eq!(
        judge_client_version(Some("0.2.0"), &table, 5_000_000_000_000),
        VersionVerdict::Accept
    );
}

#[test]
fn version_release_six_months_after_uses_calendar_months() {
    let start = 1_709_732_800_000;
    let end = crate::version_release::six_months_after(start);
    let days = (end - start) / 86_400_000;
    assert!((182..=184).contains(&days), "days={days}");
}

#[test]
fn version_release_first_release_after_skips_equal_and_older() {
    let table = table(&[("0.1.0", 10), ("0.3.0", 30), ("0.2.0", 20)]);
    let late = 30 + 210 * 86_400_000;
    assert_eq!(
        judge_client_version(Some("0.2.0"), &table, late),
        VersionVerdict::TooOld
    );
    assert_eq!(
        judge_client_version(Some("0.3.1"), &table, late),
        VersionVerdict::Accept
    );
}
