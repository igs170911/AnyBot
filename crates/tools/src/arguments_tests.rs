use super::*;

#[test]
fn test_arguments_reject_a_tool_that_is_not_in_this_turn() {
    let parsed = parse_call("browser", "{}");
    let Err(ArgsRejection::UnknownTool { tool, available }) = parsed else {
        panic!("unknown tool not rejected: {parsed:?}");
    };
    assert_eq!(tool, "browser");
    assert_eq!(
        available,
        vec![
            "edit_file".to_owned(),
            "read_file".to_owned(),
            "shell".to_owned(),
            "list_dir".to_owned(),
            "search".to_owned(),
            "read_result".to_owned(),
            "ask_person".to_owned(),
        ]
    );
}

#[test]
fn test_arguments_reject_text_that_is_not_json() {
    let parsed = parse_call(TOOL_SHELL, "not json");
    assert!(matches!(parsed, Err(ArgsRejection::Malformed { .. })));
}

#[test]
fn test_arguments_require_every_field_of_a_call() {
    let parsed = parse_call(TOOL_EDIT_FILE, r#"{"path":"a.txt","old_text":"x"}"#);
    let Err(ArgsRejection::MissingField { field, tool }) = parsed else {
        panic!("missing field not rejected: {parsed:?}");
    };
    assert_eq!(field, "new_text");
    assert_eq!(tool, "edit_file");
}

#[test]
fn test_arguments_read_optional_numbers_with_defaults() {
    let parsed = parse_call(TOOL_READ_FILE, r#"{"path":"a.txt"}"#);
    let Ok(ToolArgs::Read(args)) = parsed else {
        panic!("read args missing: {parsed:?}");
    };
    assert_eq!(args.offset, 0);
    assert_eq!(args.limit, 2000);
}

#[test]
fn search_optional_fields_default_to_workdir_and_lines() {
    let parsed = parse_call(TOOL_SEARCH, r#"{"pattern":"foo"}"#);
    let Ok(ToolArgs::Search(args)) = parsed else {
        panic!("search args missing: {parsed:?}");
    };
    assert_eq!(args.pattern, "foo");
    assert_eq!(args.path, ".");
    assert!(!args.files_only);

    let full = parse_call(
        TOOL_SEARCH,
        r#"{"pattern":"bar","path":"src","files_only":true}"#,
    );
    let Ok(ToolArgs::Search(full_args)) = full else {
        panic!("search args missing: {full:?}");
    };
    assert_eq!(full_args.path, "src");
    assert!(full_args.files_only);
}

#[test]
fn shell_rejects_old_line_parameter() {
    let parsed = parse_call(TOOL_SHELL, r#"{"line":"ls"}"#);
    let Err(ArgsRejection::MissingField { field, tool }) = parsed else {
        panic!("old line param not rejected: {parsed:?}");
    };
    assert_eq!(field, "command");
    assert_eq!(tool, "shell");
}

#[test]
fn check_offered_rejects_unoffered_and_computer_param() {
    let offered = [TOOL_READ_FILE, TOOL_SHELL];
    let missing = check_offered(TOOL_EDIT_FILE, "{}", &offered);
    assert_eq!(
        missing,
        Err(ArgsRejection::NotInTurn {
            tool: TOOL_EDIT_FILE.to_owned(),
            available: vec![TOOL_READ_FILE.to_owned(), TOOL_SHELL.to_owned()],
        })
    );
    let routed = check_offered(
        TOOL_SHELL,
        r#"{"command":"ls","computer":"Linux"}"#,
        &offered,
    );
    assert_eq!(routed, Err(ArgsRejection::OtherComputer));
    assert_eq!(
        check_offered(TOOL_SHELL, r#"{"command":"ls"}"#, &offered),
        Ok(())
    );
}
