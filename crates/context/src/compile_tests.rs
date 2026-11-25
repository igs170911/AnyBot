use super::compile;

#[test]
fn test_compile_starts_empty_input() {
    let input = crate::compile_tests_support::base_input();
    let out = compile(&input);
    assert!(out.is_sent());
}
