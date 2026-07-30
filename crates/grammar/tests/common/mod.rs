#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use config::extension::ext;
use grammar::{
    FindNodeResult, GrammarEngine, GrammarError, LoadedLanguage, ParseSession, QueryMatch,
};

pub const NONE_RANGE: Option<std::ops::Range<usize>> = None::<std::ops::Range<usize>>;

pub fn engine_with_rust() -> &'static GrammarEngine {
    static ENGINE: OnceLock<GrammarEngine> = OnceLock::new();
    ENGINE.get_or_init(|| {
        GrammarEngine::from_languages(vec![LoadedLanguage {
            id: "rust".to_string(),
            extensions: vec![ext("rs")],
            language: tree_sitter_rust::LANGUAGE.into(),
        }])
    })
}

pub fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|x| x.parent())
        .map(|p| p.join("fixtures").join("rust").join("rust.rs"))
        .expect("Failed to resolve the workspace root path")
}

pub fn run_query<R>(
    path: &Path,
    query_str: &str,
    range: Option<R>,
) -> Result<Vec<QueryMatch>, GrammarError>
where
    R: std::ops::RangeBounds<usize>,
{
    let engine = engine_with_rust();
    let source = std::fs::read_to_string(path).expect("fixture source should be readable");
    let path_str = path.to_str().expect("fixture path must be valid UTF-8");
    let lang = engine.resolve_language(path_str, Some("rust"))?;
    ParseSession::new(lang, source)?.run_query(query_str, range)
}

pub fn run(query_str: &str) -> Result<Vec<QueryMatch>, GrammarError> {
    let path = fixture_path();
    run_query(&path, query_str, NONE_RANGE)
}

pub fn run_success(query_str: &str) -> Vec<QueryMatch> {
    run(query_str).expect("query should succeed but failed")
}

pub fn find_node(byte: usize) -> Result<FindNodeResult, GrammarError> {
    let path = fixture_path();
    let source = std::fs::read_to_string(&path).expect("fixture source should be readable");
    let path_str = path.to_str().expect("fixture path must be valid UTF-8");
    let lang = engine_with_rust().resolve_language(path_str, Some("rust"))?;
    ParseSession::new(lang, source)?.find_node(byte)
}

pub fn fixture_source() -> String {
    std::fs::read_to_string(fixture_path()).expect("fixture source should be readable")
}

pub fn json_value<T: serde::Serialize>(value: &T) -> serde_json::Value {
    serde_json::to_value(value).expect("value should serialize to JSON")
}
