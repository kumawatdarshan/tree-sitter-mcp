mod common;

use insta::assert_json_snapshot;
use quickcheck_macros::quickcheck;

#[test]
fn finds_named_function_identifier() {
    let source = common::fixture_source();
    let byte = source
        .find("fetch_data")
        .expect("fixture should contain fetch_data");

    let result = common::find_node(byte).expect("function name byte should resolve");
    assert_json_snapshot!(result);
}

#[quickcheck]
fn find_node_ok_iff_byte_within_len(source: String, byte: usize) {
    let result = common::find_node_in_source(&source, byte);

    if byte <= source.len() {
        assert!(
            result.is_ok(),
            "in-bounds byte {byte} on {source:?} should resolve"
        );
    } else {
        assert!(
            matches!(result, Err(grammar::GrammarError::ByteOutOfBounds { .. })),
            "out-of-bounds byte {byte} on {source:?} should be rejected"
        );
    }
}

#[quickcheck]
fn in_bounds_byte_returns_rooted_chain(source: String, byte: usize) {
    let byte = byte % (source.len() + 1);
    let value = common::json_value(
        &common::find_node_in_source(&source, byte).expect("in-bounds byte should resolve"),
    );
    let ancestors = value["ancestors"]
        .as_array()
        .expect("ancestors should be an array");

    assert!(!ancestors.is_empty());
    assert!(matches!(
        ancestors.last().unwrap()["kind"].as_str(),
        Some("source_file" | "ERROR")
    ));
}
