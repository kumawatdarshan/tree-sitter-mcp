use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use std::path::PathBuf;
use tree_sitter::{Parser, Query, QueryCursor, StreamingIterator};

fn rust_fixture() -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|x| x.parent())
        .map(|p| p.join("fixtures").join("rust").join("rust.rs"))
        .expect("workspace root should resolve");

    std::fs::read_to_string(path).expect("rust fixture should be readable")
}

fn parse_rust(source: &str) -> tree_sitter::Tree {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_rust::LANGUAGE.into())
        .expect("rust language should load");
    parser
        .parse(source, None)
        .expect("rust fixture should parse")
}

fn count_matches(query: &Query, tree: &tree_sitter::Tree, source: &str) -> usize {
    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(query, tree.root_node(), source.as_bytes());
    let mut count = 0;

    while matches.next().is_some() {
        count += 1;
    }

    count
}

fn query_benchmarks(c: &mut Criterion) {
    let source = rust_fixture();
    let language = tree_sitter_rust::LANGUAGE.into();
    let tree = parse_rust(&source);

    let simple = Query::new(&language, "(function_item name: (identifier) @name)")
        .expect("simple query should compile");
    let multi_capture = Query::new(
        &language,
        r#"(function_item name: (identifier) @fn
             (parameters (parameter pattern: (identifier) @param)))"#,
    )
    .expect("multi-capture query should compile");
    let no_match = Query::new(&language, "(macro_definition name: (identifier) @name)")
        .expect("no-match query should compile");

    c.bench_function("parse_rust_fixture", |b| {
        b.iter(|| parse_rust(black_box(&source)))
    });

    c.bench_function("query_function_names", |b| {
        b.iter(|| count_matches(black_box(&simple), black_box(&tree), black_box(&source)))
    });

    c.bench_function("query_function_parameters", |b| {
        b.iter(|| {
            count_matches(
                black_box(&multi_capture),
                black_box(&tree),
                black_box(&source),
            )
        })
    });

    c.bench_function("query_no_matches", |b| {
        b.iter(|| count_matches(black_box(&no_match), black_box(&tree), black_box(&source)))
    });
}

criterion_group!(benches, query_benchmarks);
criterion_main!(benches);
