mod common;

use crate::common::NONE_RANGE;
use insta::assert_json_snapshot;
use quickcheck_macros::quickcheck;
use rstest::rstest;

#[rstest]
#[case::function_names("function_names", "(function_item name: (identifier) @name)")]
#[case::struct_names("struct_names", "(struct_item name: (type_identifier) @name)")]
#[case::enum_names("enum_names", "(enum_item name: (type_identifier) @name)")]
#[case::trait_names("trait_names", "(trait_item name: (type_identifier) @name)")]
#[case::const_names("const_names", "(const_item name: (identifier) @name)")]
#[case::static_names("static_names", "(static_item name: (identifier) @name)")]
#[case::type_alias_names("type_alias_names", "(type_item name: (type_identifier) @name)")]
fn finds_item_names(#[case] case_name: &str, #[case] query: &str) {
    insta::glob!("../../..", "fixtures/rust/*", |path| {
        let matches = common::run_query(path, query, NONE_RANGE).expect("query should succeed");
        assert_json_snapshot!(format!("finds_item_names__{case_name}"), matches);
    });
}

#[test]
fn finds_async_function_with_attributes() {
    let matches = common::run_success(
        r#"((attribute_item) @attr
           .
           (function_item name: (identifier) @name))"#,
    );
    let names: Vec<_> = matches
        .iter()
        .filter_map(|m| {
            let value = common::json_value(m);
            let captures = value["captures"]
                .as_array()
                .expect("captures should be an array");

            let attr = captures
                .iter()
                .find(|capture| capture["name"] == "attr")
                .and_then(|capture| capture["node"]["text"].as_str())?;

            if !attr.contains("tracing") {
                return None;
            }

            captures
                .iter()
                .find(|capture| capture["name"] == "name")
                .and_then(|capture| capture["node"]["text"].as_str())
                .map(str::to_string)
        })
        .collect();

    assert_eq!(
        names,
        ["instrumented_function", "instrumented_with_options"]
    );
}

#[test]
fn finds_function_parameters() {
    let matches = common::run_success(
        "(function_item name: (identifier) @fn
           (parameters (parameter pattern: (identifier) @param)))",
    );
    assert_json_snapshot!(matches);
}

#[test]
#[ignore = "#match? predicate support requires QueryCursor::set_match_predicate"]
fn predicate_filter_identifies_async_items() {
    let matches =
        common::run_success(r#"(function_item name: (identifier) @name) (#match? @name "^fetch")"#);
    assert_eq!(matches.len(), 1);
    assert_json_snapshot!(matches);
}

#[test]
fn captures_no_matches_for_nonexistent_pattern() {
    let matches = common::run_success("(macro_definition name: (identifier) @name)");
    assert!(matches.is_empty());
}

#[rstest]
#[case::unclosed_paren("(unclosed-pattern")]
fn invalid_query_returns_query_error(#[case] query: &str) {
    let err = common::run(query).expect_err("query should fail");
    match err {
        grammar::GrammarError::Query(_) => {}
        other => panic!("expected GrammarError::Query, got {other:?}"),
    }
}

#[quickcheck]
fn range_limited_query_never_exceeds_full(end: usize) {
    let full = common::run_success("(struct_item) @s");

    let end = end % 5000 + 1;
    let limited = common::run_query(&common::fixture_path(), "(struct_item) @s", Some(0..end))
        .expect("query should succeed");
    assert!(limited.len() <= full.len());
}

#[quickcheck]
fn range_limited_function_query_never_exceeds_full(end: usize) {
    let source = common::fixture_source();
    let full = common::run_success("(function_item name: (identifier) @name)");

    let end = end % (source.len() + 1);
    let limited = common::run_query(
        &common::fixture_path(),
        "(function_item name: (identifier) @name)",
        Some(0..end),
    )
    .expect("query should succeed");

    assert!(limited.len() <= full.len());
}

#[quickcheck]
fn malformed_queries_never_panic(prefix: String, open_parens: usize) {
    let prefix: String = prefix.chars().take(40).collect();
    let open_parens = open_parens % 5 + 1;
    let query = format!("{}{}", "(".repeat(open_parens), prefix);
    let result = common::run(&query);
    assert!(result.is_err());
}
