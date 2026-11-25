use super::*;

fn set(items: &[&str]) -> BTreeSet<String> {
    items.iter().map(ToString::to_string).collect()
}

#[test]
fn test_spec_trace_collects_table_ids() {
    let md = "| id | kind | given | when | expect |\n\
              |---|---|---|---|---|\n\
              | T-chat-steer-1 | e2e | a | b | c |\n\
              | T-chat-steer-2 | integration | a | b | c |\n\
              the T-chat-steer-9 mentioned in prose sits outside table cells and stays out of the list.\n";
    let ids = spec_ids_from_md(md).unwrap();
    assert!(ids.contains("T-chat-steer-1"));
    assert!(ids.contains("T-chat-steer-2"));
    assert!(!ids.contains("T-chat-steer-9"));
}

#[test]
fn test_spec_trace_rust_test_names_are_the_trace_source() {
    let src = "#[test]\nfn t_chat_steer_1() {}\n\
               #[tokio::test]\nasync fn t_chat_steer_2() {}\n\
               fn helper_not_a_test() {}\n\
               #[test]\nfn notes() {}\n";
    let ids = code_ids_from_source(src).unwrap();
    assert!(ids.contains("T-chat-steer-1"));
    assert!(ids.contains("T-chat-steer-2"));
    assert_eq!(ids.len(), 2);
}

#[test]
fn test_spec_trace_rust_dashes_underscore_mapping() {
    let src = "#[test]\nfn t_chat_steer_1() {}\n#[test]\nfn t_x_2b() {}\n";
    let ids = code_ids_from_source(src).unwrap();
    assert!(ids.contains("T-chat-steer-1"));
    assert!(ids.contains("T-x-2b"));
}

#[test]
fn test_spec_trace_ts_test_titles_are_the_trace_source() {
    let ts = "test(\"T-chat-steer-1\", () => {});\n\
              it('T-chat-steer-2', () => {});\n\
              test(\"T-no-prefix\", () => {});\n";
    let ids = code_ids_from_ts_source(ts).unwrap();
    assert!(ids.contains("T-chat-steer-1"));
    assert!(ids.contains("T-chat-steer-2"));
    assert_eq!(ids.len(), 3);
    assert!(ids.contains("T-no-prefix"));
}

#[test]
fn test_spec_trace_finds_indented_and_macro_tests() {
    let src = "mod tests {\n    use super::*;\n\n    #[test]\n    fn t_sync_id_1() {}\n\n    #[test]\n    /// doc comment\n    fn t_chat_steer_3() {}\n\n    proptest! {\n        #[test]\n        fn t_sync_seq_1(v in 0..64u64) {\n            let _ = v;\n        }\n    }\n";
    let ids = code_ids_from_source(src).unwrap();
    assert!(ids.contains("T-sync-id-1"));
    assert!(ids.contains("T-chat-steer-3"));
    assert!(ids.contains("T-sync-seq-1"));
    assert_eq!(ids.len(), 3);
}

#[test]
fn test_spec_trace_blocks_unknown_id() {
    let baseline = set(&["T-chat-steer-1"]);
    let r = check(
        &set(&["T-chat-steer-1"]),
        &set(&["T-nope-1"]),
        Some(&baseline),
    );
    assert!(
        r.violations
            .iter()
            .any(|v| v.contains("test name carries an id absent from the spec: T-nope-1"))
    );
}

#[test]
fn test_spec_trace_baseline_only_shrinks() {
    let baseline = set(&["T-chat-steer-1", "T-chat-steer-2"]);
    let r = check(
        &set(&["T-chat-steer-1", "T-chat-steer-2"]),
        &set(&["T-chat-steer-1"]),
        Some(&baseline),
    );
    assert!(
        r.violations
            .iter()
            .any(|v| v.contains("T-chat-steer-1 already has a test and is still in the baseline"))
    );
    assert_eq!(r.total, 2);
    assert_eq!(r.tested(), 1);
    assert_eq!(r.missing(), 1);
}

#[test]
fn test_spec_trace_new_id_must_be_tracked() {
    let baseline = set(&["T-chat-steer-1"]);
    let r = check(
        &set(&["T-chat-steer-1", "T-chat-steer-2"]),
        &set(&["T-chat-steer-1"]),
        Some(&baseline),
    );
    assert!(
        r.violations
            .iter()
            .any(|v| v.contains("spec added id T-chat-steer-2"))
    );
}

#[test]
fn test_spec_trace_missing_baseline_fails() {
    let r = check(
        &set(&["T-chat-steer-1", "T-chat-steer-2"]),
        &set(&["T-chat-steer-1"]),
        None,
    );
    assert!(
        r.violations
            .iter()
            .any(|v| v.contains("baseline tools/spec-trace/baseline.txt is missing"))
    );
}

#[test]
fn test_spec_trace_generate_baseline_excludes_tested() {
    let g = generate_baseline(
        &set(&["T-chat-steer-1", "T-chat-steer-2"]),
        &set(&["T-chat-steer-1"]),
    );
    assert!(g.contains("T-chat-steer-2"));
    assert!(!g.contains("T-chat-steer-1"));
}

#[test]
fn test_spec_trace_against_ref_allows_shrink_and_new_spec_ids() {
    let v = check_baseline_against_ref(
        &set(&["T-chat-steer-2"]),
        &set(&["T-chat-steer-1", "T-chat-steer-2", "T-chat-steer-3"]),
        &set(&["T-chat-steer-4"]),
    );
    assert_eq!(v, Vec::<String>::new());
    let v2 = check_baseline_against_ref(
        &set(&["T-chat-steer-1", "T-chat-steer-4"]),
        &set(&["T-chat-steer-1"]),
        &set(&["T-chat-steer-4"]),
    );
    assert_eq!(v2, Vec::<String>::new());
}

#[test]
fn test_spec_trace_against_ref_blocks_stuffed_back_id() {
    let v = check_baseline_against_ref(&set(&["T-chat-steer-1"]), &set(&[]), &set(&[]));
    assert_eq!(v.len(), 1);
    let first = v.first().unwrap();
    assert!(first.contains("T-chat-steer-1"));
    assert!(first.contains("may only shrink"));
}
