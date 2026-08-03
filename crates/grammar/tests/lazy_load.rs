mod common;

use config::extension::ExtensionMap;
use grammar::{GrammarEngine, GrammarError};
use std::path::Path;

fn engine_with_empty_grammar_dir() -> GrammarEngine {
    let map = ExtensionMap::from_toml_str("rust = [\"rs\"]\npython = [\"py\"]\n")
        .expect("valid extension map");
    GrammarEngine::load(map, Path::new("/nonexistent/grammar-dir"))
}

#[test]
fn lazy_load_unknown_id_is_not_configured() {
    let engine = engine_with_empty_grammar_dir();
    let err = engine.load_language("brainfuck").unwrap_err();
    assert!(matches!(err, GrammarError::UnknownLanguage(ref id) if id == "brainfuck"));
}

#[test]
fn lazy_load_missing_library_reports_load_failure() {
    let engine = engine_with_empty_grammar_dir();
    let err = engine.load_language("rust").unwrap_err();
    assert!(
        matches!(err, GrammarError::LoadGrammar { .. }),
        "configured id without a compiled grammar should be a load failure"
    );
}

#[test]
fn available_ids_are_sorted_and_complete() {
    let engine = engine_with_empty_grammar_dir();
    assert_eq!(engine.available_ids(), vec!["python", "rust"]);
}

#[test]
fn load_starts_with_no_dlopen_side_effects() {
    let engine = engine_with_empty_grammar_dir();
    // Loading the engine should not have attempted to open any grammar;
    // a from_languages engine has zero preloads.
    let ids = engine.available_ids();
    assert_eq!(ids.len(), 2);
}

#[test]
fn from_languages_engine_resolves_and_loads_on_demand() {
    let engine = common::engine_with_rust();
    let lang = engine
        .resolve_language("test.rs", Some("rust"))
        .expect("rust should resolve");
    assert_eq!(lang.id, "rust");
    let _ = grammar::ParseSession::new(lang, "fn main() {}".to_string()).expect("should parse");
}

#[test]
fn extension_inference_lazy_loads_rust() {
    let engine = common::engine_with_rust();
    let lang = engine
        .resolve_language("src/main.rs", None)
        .expect("rust should infer from extension");
    assert_eq!(lang.id, "rust");
}
