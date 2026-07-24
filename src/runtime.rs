use std::path::{Path, PathBuf};

use etcetera::base_strategy::{choose_base_strategy, BaseStrategy};

const RUNTIME_DIR_NAME: &str = "runtime";

/// Resolve the runtime directory.
///
/// Priority:
/// 1. An explicit path passed by the caller (from config's `grammar_dir`)
/// 2. `TREE_SITTER_MCP_RUNTIME` environment variable
/// 3. XDG data home → `~/.local/share/tree-sitter-mcp/runtime/`
///
/// No workspace or per-directory overrides — unlike Helix, this is a server,
/// not an editor.
pub fn runtime_dir(explicit: Option<&Path>) -> PathBuf {
    if let Some(dir) = explicit {
        return dir.to_path_buf();
    }

    if let Ok(dir) = std::env::var("TREE_SITTER_MCP_RUNTIME") {
        return PathBuf::from(dir);
    }

    let strategy = choose_base_strategy().expect("unable to determine base directories");
    let mut path = strategy.data_dir();
    path.push("tree-sitter-mcp");
    path.push(RUNTIME_DIR_NAME);
    path
}

/// Find a file relative to the runtime directory.
///
/// Returns `Some(path)` if the file exists, `None` otherwise.
pub fn find_runtime_file(explicit: Option<&Path>, rel_path: &Path) -> Option<PathBuf> {
    let path = runtime_dir(explicit).join(rel_path);
    if path.exists() {
        Some(path)
    } else {
        None
    }
}

/// Load a language-specific runtime file (e.g. queries).
///
/// Looks for `<runtime_dir>/queries/<language>/<filename>`.
pub fn load_runtime_file(
    explicit: Option<&Path>,
    language: &str,
    filename: &str,
) -> Result<String, std::io::Error> {
    let path = runtime_dir(explicit)
        .join("queries")
        .join(language)
        .join(filename);
    std::fs::read_to_string(path)
}