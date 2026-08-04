mod common;

use grammar::LanguageId;
use insta::assert_snapshot;

#[test]
fn dump_ast_preserves_tree_sitter_error_recovery() {
    let source = common::fixture_source();
    let lang = common::engine_with_rust()
        .resolve_language("test.rs", Some(&LanguageId::new("rust").unwrap()))
        .expect("language should resolve");

    let ast = grammar::ParseSession::new(lang, source)
        .expect("parse should succeed")
        .dump_ast(common::NONE_RANGE);

    assert_snapshot!("dump_ast_preserves_tree_sitter_error_recovery", ast);
}

#[test]
fn dump_ast_can_be_limited_to_a_byte_range() {
    let source = common::fixture_source();
    let start = source
        .find("let p = Point::new")
        .expect("fixture should contain fetch_data body");
    let end = source[start..]
        .find("Ok(url")
        .map(|offset| start + offset)
        .expect("fixture should contain fetch_data body end");

    let lang = common::engine_with_rust()
        .resolve_language("test.rs", Some(&LanguageId::new("rust").unwrap()))
        .expect("language should resolve");

    let session = grammar::ParseSession::new(lang, source).expect("parse should succeed");
    let full = session.dump_ast(common::NONE_RANGE);
    let limited = session.dump_ast(Some(start..end));

    assert!(
        full.contains(&limited),
        "range-limited dump should be a sub-expression of the full dump"
    );
    assert!(
        limited.len() < full.len(),
        "range-limited dump should actually truncate"
    );
}
