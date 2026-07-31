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
    // The fixture's buggy tail makes the tree root an ERROR node, which
    // breaks top-level `.` (sibling-adjacency) anchoring. Query the valid
    // portion, where item adjacency is reliable.
    let source = common::fixture_source();
    let valid = source
        .split("// BUGGY PORTION")
        .next()
        .expect("fixture should contain the buggy-portion marker");

    let lang = common::engine_with_rust()
        .resolve_language("test.rs", Some("rust"))
        .expect("language should resolve");

    let matches = grammar::ParseSession::new(lang.clone(), valid.to_string())
        .expect("parse should succeed")
        .run_query(
            r#"((attribute_item) @attr
               .
               (function_item name: (identifier) @name))"#,
            common::NONE_RANGE,
        )
        .expect("query should succeed");

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
        [
            "fetch_data",
            "linux_only_system_call",
            "windows_only_system_call",
            "process_extra_metrics",
            "instrumented_function",
            "instrumented_with_options"
        ]
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

#[rstest]
#[case::nonexistent_node_type("(macro_definition name: (identifier) @name)")]
#[case::empty_query("")]
#[case::whitespace_only("   ")]
fn no_match_queries(#[case] query: &str) {
    let matches = common::run_success(query);
    assert!(matches.is_empty());
}

#[rstest]
#[case::unclosed_paren("(unclosed-pattern")]
#[case::stray_close_paren(")")]
#[case::bare_capture("@")]
#[case::unclosed_predicate_string(
    r#"(function_item name: (identifier) @name) (#eq? @name "unclosed)"#
)]
fn invalid_query_returns_query_error(#[case] query: &str) {
    let err = common::run(query).expect_err("query should fail");
    match err {
        grammar::GrammarError::Query(_) => {}
        other => panic!("expected GrammarError::Query, got {other:?}"),
    }
}

#[quickcheck]
fn range_limited_query_never_exceeds_full(end: usize) {
    let source = common::fixture_source();
    let end = end % (source.len() + 1);

    for query in [
        "(struct_item) @s",
        "(function_item name: (identifier) @name)",
    ] {
        let full = common::run_success(query);
        let limited = common::run_query(&common::fixture_path(), query, Some(0..end))
            .expect("query should succeed");
        assert!(limited.len() <= full.len());
    }
}

#[quickcheck]
fn malformed_queries_never_panic(prefix: String, open_parens: usize) {
    // Strip `;` so a random prefix can't start a query-language comment that
    // swallows the trailing parens. With trailing (unbalanced) parens the
    // query is invalid by construction.
    let prefix: String = prefix.chars().take(40).filter(|c| *c != ';').collect();
    let open_parens = open_parens % 5 + 1;
    let query = format!("{prefix}{}", "(".repeat(open_parens));
    let result = common::run(&query);
    assert!(
        result.is_err(),
        "unbalanced query should fail, got success for {query:?}"
    );
}
