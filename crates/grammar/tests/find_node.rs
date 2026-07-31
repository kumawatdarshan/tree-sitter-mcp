mod common;

use quickcheck_macros::quickcheck;

#[test]
fn finds_ancestor_chain_at_start_of_file() {
    let result = common::find_node(0).expect("byte 0 should resolve");
    let value = common::json_value(&result);
    let ancestors = value["ancestors"]
        .as_array()
        .expect("ancestors should be an array");

    assert!(!ancestors.is_empty());
    assert!(matches!(
        ancestors.last().unwrap()["kind"].as_str(),
        Some("source_file" | "ERROR")
    ));
}

#[test]
fn finds_named_function_identifier() {
    let source = common::fixture_source();
    let byte = source
        .find("fetch_data")
        .expect("fixture should contain fetch_data");

    let result = common::find_node(byte).expect("function name byte should resolve");
    let value = common::json_value(&result);
    let ancestors = value["ancestors"]
        .as_array()
        .expect("ancestors should be an array");

    assert_eq!(ancestors.first().unwrap()["kind"], "identifier");
    assert_eq!(ancestors.first().unwrap()["text"], "fetch_data");
    assert!(ancestors.iter().any(|node| node["kind"] == "function_item"));
    assert!(matches!(
        ancestors.last().unwrap()["kind"].as_str(),
        Some("source_file" | "ERROR")
    ));
}

#[test]
fn rejects_byte_past_end_of_file() {
    let source = common::fixture_source();
    let err = common::find_node(source.len() + 1).expect_err("byte past EOF should fail");

    assert!(matches!(
        err,
        grammar::GrammarError::ByteOutOfBounds { byte, len }
            if byte == source.len() + 1 && len == source.len()
    ));
}

#[quickcheck]
fn in_bounds_byte_returns_non_empty_chain(byte: usize) {
    let source = common::fixture_source();
    let byte = byte % (source.len() + 1);
    let result = common::find_node(byte).expect("in-bounds byte should resolve");
    let value = common::json_value(&result);
    let ancestors = value["ancestors"]
        .as_array()
        .expect("ancestors should be an array");

    assert!(!ancestors.is_empty());
    assert!(matches!(
        ancestors.last().unwrap()["kind"].as_str(),
        Some("source_file" | "ERROR")
    ));
}
