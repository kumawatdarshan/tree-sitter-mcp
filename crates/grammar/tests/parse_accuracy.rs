mod common;

#[test]
#[ignore]
fn dump_ast_returns_source_file_sexp() {
    let source = common::fixture_source();
    let lang = common::engine_with_rust()
        .resolve_language("test.rs", Some("rust"))
        .expect("language should resolve");

    let ast = grammar::ParseSession::new(lang.clone(), source)
        .expect("parse should succeed")
        .dump_ast(common::NONE_RANGE);

    assert!(ast.contains("source_file"), "expected source_file in ast");
    assert!(
        ast.contains("function_item"),
        "expected function_item in ast"
    );
}

#[test]
fn dump_ast_preserves_tree_sitter_error_recovery() {
    let source = common::fixture_source();
    let lang = common::engine_with_rust()
        .resolve_language("test.rs", Some("rust"))
        .expect("language should resolve");

    let ast = grammar::ParseSession::new(lang.clone(), source)
        .expect("parse should succeed")
        .dump_ast(common::NONE_RANGE);

    assert!(ast.contains("ERROR"));
}

#[test]
#[ignore]
fn dump_ast_can_be_limited_to_a_byte_range() {
    let source = common::fixture_source();
    let start = source
        .find("pub async fn fetch_data")
        .expect("fixture should contain fetch_data");
    let end = source[start..]
        .find("pub fn legacy_calculator")
        .map(|offset| start + offset)
        .expect("fixture should contain following function");

    let lang = common::engine_with_rust()
        .resolve_language("test.rs", Some("rust"))
        .expect("language should resolve");

    let ast = grammar::ParseSession::new(lang.clone(), source)
        .expect("parse should succeed")
        .dump_ast(Some(start..end));

    assert!(ast.contains("fetch_data"));
    assert!(!ast.contains("legacy_calculator"));
}
