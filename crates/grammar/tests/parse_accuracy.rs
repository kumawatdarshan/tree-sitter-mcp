mod common;

#[test]
fn dump_ast_returns_source_file_sexp() {
    let path = common::fixture_path();
    let path_str = path.to_str().expect("fixture path must be valid UTF-8");

    let ast = common::engine_with_rust()
        .open(path_str, Some("rust"))
        .expect("open should succeed")
        .dump_ast(common::NONE_RANGE);

    assert!(ast.contains("source_file"));
    assert!(ast.contains("function_item"));
}

#[test]
fn dump_ast_preserves_tree_sitter_error_recovery() {
    let path = common::fixture_path();
    let path_str = path.to_str().expect("fixture path must be valid UTF-8");

    let ast = common::engine_with_rust()
        .open(path_str, Some("rust"))
        .expect("open should succeed")
        .dump_ast(common::NONE_RANGE);

    assert!(ast.contains("ERROR"));
}

#[test]
fn dump_ast_can_be_limited_to_a_byte_range() {
    let source = common::fixture_source();
    let start = source
        .find("pub async fn fetch_data")
        .expect("fixture should contain fetch_data");
    let end = source[start..]
        .find("pub fn legacy_calculator")
        .map(|offset| start + offset)
        .expect("fixture should contain following function");
    let path = common::fixture_path();
    let path_str = path.to_str().expect("fixture path must be valid UTF-8");

    let ast = common::engine_with_rust()
        .open(path_str, Some("rust"))
        .expect("open should succeed")
        .dump_ast(Some(start..end));

    assert!(ast.contains("fetch_data"));
    assert!(!ast.contains("legacy_calculator"));
}
