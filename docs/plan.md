# Plan: Tree-Sitter MCP Server

A general-purpose, multi-language tree-sitter MCP server in Rust. Runtime-loaded grammars from the server's own grammar directory. 18 tools (16 semantic + 2 building blocks). stdio transport. Primary consumer: AI coding agents.

**API Contract:** [`./API.md`](./API.md)

## Dependencies

```toml
[dependencies]
anyhow = "1.0"
rmcp = { version = "2.2", features = ["server", "transport-io", "macros"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
thiserror = "2.0"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
tree-sitter = "0.26"
dlopen2 = { version = "0.9", default-features = false, features = ["symbor"] }  # safe dlopen for grammar libraries
toml = "0.8"                 # runtime config parsing
dirs = "6.0"                 # XDG paths for the server's own grammar directory
```

### Test dependencies

```toml
[dev-dependencies]
insta = { version = "1.0", features = ["json", "glob"] }
rstest = "0.26"
quickcheck = "1"
quickcheck_macros = "1"
criterion = { version = "0.8", features = ["html_reports"] }
```

Testing split:
- Fixed query examples use `rstest` plus `insta` snapshots.
- Property-style invariants use `quickcheck`; do not reintroduce `proptest` unless a future invariant needs shrinking or custom generators that quickcheck cannot express cleanly.
- Rich language fixtures live under `fixtures/<language>/` and should cover normal syntax, language-specific attributes/modifiers, and parser-recovery cases.
- Empty query strings are valid tree-sitter queries that produce zero matches.
- Unknown predicates are not query-compile errors; predicate semantics must be implemented by the query executor when required.

No `tree-house`. We dlopen compiled grammar libraries directly via `dlopen2`. Each library exports a C function `tree_sitter_<language>()` that returns a `tree_sitter::Language`. The symbol name is derived from the library filename: the stem with `-` mapped to `_` (e.g. `c-sharp.so` → `tree_sitter_c_sharp`).

## Architecture

```
src/
  main.rs              — entry: parse args, init tracing, load config, run MCP server
  server.rs            — rmcp server struct, tool registration, ServerHandler impl
  config.rs            — runtime config: extension→language map, grammar dir, etc.
  grammar/
    mod.rs             — GrammarManager: dlopen .so, cache Language values
    dlopen.rs          — unsafe libloading wrapper for tree_sitter_* symbols
  tools/
    mod.rs             — single `parse` tool dispatch (action param)
    ast.rs             — cursor-based tree→JSON serialization (named nodes only)
    query.rs           — run tree-sitter S-expression queries
    symbols.rs         — extract named definitions (functions, classes, etc.)
    node_at.rs         — find node at position (line:col)
    references.rs      — find usages of a symbol
  prompts/
    mod.rs             — NL → S-expression prompt content
    query_patterns.rs  — per-language query pattern tables
  error.rs             — structured error types for agent-readable messages
```

## Runtime Config

The extension→language map and grammar directory are runtime-configurable via a TOML file. The server looks for config in this order:

1. `--grammar-dir <DIR>` CLI argument
2. `TREE_SITTER_MCP_GRAMMAR_DIR` environment variable
3. `grammar_dir` key in `~/.config/tree-sitter-mcp/languages.toml` (XDG config dir)
4. Built-in default: `~/.local/share/tree-sitter-mcp/grammars` (XDG data dir)

> The `grammar_dir` key must appear **before** the `[extensions]` table in the TOML, otherwise it is parsed as a member of `[extensions]`.

### Config file format

```toml
# grammar_dir: where to find compiled grammar libraries (.so/.dylib/.dll)
# Default: the server's own XDG data directory — ~/.local/share/tree-sitter-mcp/grammars
grammar_dir = "/absolute/path/to/grammars"

[extensions]
# Maps language id → file extensions / glob patterns
# The language id must match a grammar library stem (e.g. "rust" → "rust.so", "c-sharp" → "c-sharp.so")
rust = ["rs"]
python = ["py"]
c-sharp = ["cs", "csx", "cake"]
markdown = ["md", { glob = "PULLREQ_EDITMSG" }]
```

If no config file is found, the server uses this built-in fallback map (same entries above). Users can override any entry or add new ones by placing a config file.

### Config struct

```rust
#[derive(Deserialize)]
pub struct Config {
    pub grammar_dir: Option<String>,        // server's own grammars dir
    pub fallback_dirs: Vec<String>,         // additional grammar search dirs
    pub extensions: HashMap<String, Vec<String>>, // ext → grammar name
}
```

## Grammar Loading (`grammar/`)

### Discovery + dlopen approach

Each compiled grammar is a shared library in the server's own grammar directory (`grammar_dir`, default `~/.local/share/tree-sitter-mcp/grammars`). The loader scans **top-level** entries only, skips subdirectories (e.g. `sources/`), and keeps files matching the platform's shared-library extension (`.so` on Linux, `.dylib` on macOS, `.dll` on Windows). Each file exports a C symbol:

```
tree_sitter_<name> -> unsafe extern "C" fn() -> tree_sitter::Language
```

We load via `dlopen2::symbor`, keeping the library mapped for the process lifetime:

```rust
// crates/grammar/src/loader.rs (abridged)
let library = dlopen2::symbor::Library::open(path)?;
let constructor: dlopen2::symbor::Symbol<unsafe extern "C" fn() -> tree_sitter::Language> =
    unsafe { library.symbol(&symbol_name)? };
let language = unsafe { constructor() };
std::mem::forget(library);  // Language borrows tables/strings from the .so
```

- The symbol name is derived from the file stem: `tree_sitter_` + stem with `-` → `_`. Verified against every grammar: `c-sharp.so` → `tree_sitter_c_sharp`, `markdown_inline.so` → `tree_sitter_markdown_inline`.
- Grammars are keyed by the **normalized** name (`-` → `_`) so config ids like `c-sharp` join to `c_sharp`.
- ABI gate: only languages with `abi_version()` in `MIN_COMPATIBLE_LANGUAGE_VERSION..=LANGUAGE_VERSION` are kept.
- A file that fails to open/symbol-lookup/ABI-check is logged (`tracing::warn!`) and skipped — a single bad library can't take down the server.
- A missing/unreadable `grammar_dir` logs a warning and yields an empty map (server still starts).

### GrammarManager

```rust
// crates/grammar/src/lib.rs
pub struct GrammarEngine {
    registry: GrammarRegistry,
}
```

`GrammarEngine::load` runs three steps: `specs_from_config` (config → specs, no I/O), `discover_grammars` (scan + dlopen), `join` (match specs to discovered grammars; unmatched config entries are returned as `missing` and warned about at startup). `resolve_language(path, requested_id)` is pure, extension-first then glob fallback.

### Grammar directory resolution

The server resolves its grammar directory with this precedence:
1. `--grammar-dir <DIR>` CLI argument
2. Else `$TREE_SITTER_MCP_GRAMMAR_DIR` environment variable
3. Else the `grammar_dir` key in `languages.toml` (XDG config dir)
4. Else the default data directory: `~/.local/share/tree-sitter-mcp/grammars/` (XDG)

## Tool Surface

Single `parse` tool with `action` parameter. Registered via rmcp macros.

### Tool definition

```rust
#[derive(Serialize, Deserialize, schemars::JsonSchema)]
struct ParseParams {
    #[schemars(description = "Path to the file to parse")]
    file_path: String,

    #[schemars(description = "Language override (optional, auto-detected from extension)")]
    language: Option<String>,

    #[schemars(description = "Action to perform")]
    action: ParseAction,
}

#[derive(Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "type")]
enum ParseAction {
    /// Get the AST as named nodes
    #[serde(rename = "ast")]
    Ast { max_depth: Option<usize> },

    /// Run a tree-sitter S-expression query
    #[serde(rename = "query")]
    Query {
        query: String,
        max_results: Option<usize>,
    },

    /// Extract named definitions (functions, classes, variables, etc.)
    #[serde(rename = "symbols")]
    Symbols {
        symbol_types: Option<Vec<String>>,  // filter: ["function", "class", "struct"]
    },

    /// Find references/usages of a symbol name
    #[serde(rename = "references")]
    References {
        symbol: String,
    },

    /// Find the node at a specific line:column position
    #[serde(rename = "node_at")]
    NodeAt {
        line: usize,
        column: usize,
    },
}
```

### Tool description (what agents see)

The tool description teaches agents when to use each action:

```
Parse and analyze source code files using tree-sitter.

Actions:
- ast: Get the full AST as a tree of named nodes. Use for understanding code structure.
- query: Run a tree-sitter S-expression query. Use for precise pattern matching.
- symbols: Extract named definitions (functions, classes, structs, imports, etc.).
- references: Find all usages/references of a symbol name in a file.
- node_at: Find the AST node at a specific line:column position. Use for "what's at line X?"

Auto-detects language from file extension. Override with language param if needed.
```

### Server struct

```rust
#[derive(Clone)]
pub struct TreeSitterServer {
    grammar_manager: Arc<Mutex<GrammarManager>>,
}

#[tool_router]
impl TreeSitterServer {
    pub fn new(config: Config) -> Self {
        Self {
            grammar_manager: Arc::new(Mutex::new(GrammarManager::new(config))),
        }
    }

    #[tool(description = "Parse and analyze source code files using tree-sitter")]
    async fn parse(
        &self,
        Parameters(ParseParams { file_path, language, action }): Parameters<ParseParams>,
    ) -> Result<CallToolResult, McpError> {
        // 1. Read file from disk
        // 2. Detect or override language
        // 3. Parse with tree-sitter
        // 4. Dispatch to action handler
        // 5. Serialize result as JSON
    }
}
```

## AST Serialization (`tools/ast.rs`)

### Output format

Named nodes only. Cursor-based iterative traversal (not recursive) to handle large trees.

```json
{
  "type": "function_item",
  "name": "my_function",
  "field_name": "definition",
  "start_point": { "row": 10, "column": 0 },
  "end_point": { "row": 15, "column": 1 },
  "start_byte": 245,
  "end_byte": 380,
  "named": true,
  "children": [
    {
      "type": "identifier",
      "name": "my_function",
      "field_name": "name",
      "start_point": { "row": 10, "column": 3 },
      "end_point": { "row": 10, "column": 14 },
      "start_byte": 248,
      "end_byte": 259,
      "named": true,
      "children": []
    },
    {
      "type": "block",
      "field_name": "body",
      "start_point": { "row": 11, "column": 0 },
      "end_point": { "row": 15, "column": 1 },
      "start_byte": 262,
      "end_byte": 380,
      "named": true,
      "truncated": true,
      "children_note": "max_depth reached, 12 children hidden"
    }
  ]
}
```

### Serialization approach

```rust
pub fn node_to_json(node: tree_sitter::Node, source: &[u8], max_depth: usize) -> serde_json::Value {
    let mut stack: Vec<(tree_sitter::Node, usize, &mut Vec<serde_json::Value>)> = Vec::new();
    // ... iterative cursor-based traversal
    // Skip anonymous nodes (type.is_named() == false)
    // Record field_name via node.field_name_for_child(i)
    // Truncate at max_depth, set "truncated": true
}
```

Key decisions:
- **Named nodes only**: skip anonymous nodes (`;`, `(`, `)`, keywords that aren't named)
- **Field names**: include `field_name` when the node has one (e.g., "name", "body", "condition")
- **Depth limiting**: `max_depth` param (default 5), truncated nodes get `"truncated": true`
- **No text by default**: source text is large; agents can request it via a separate action if needed
- **Position info**: both line:column and byte offsets

## Query Support (`tools/query.rs`)

### Query action

The `query` action takes a raw tree-sitter S-expression query string, compiles it, and runs it against the parsed file.

Current scope: compile and run structural tree-sitter queries, including built-in query syntax errors and captures. Regex-style predicate filtering such as `#match?`/`#not-match?` is deferred until the query executor wires predicate support through `QueryCursor::set_match_predicate()` or an equivalent post-filtering layer.

```rust
pub fn run_query(tree: &tree_sitter::Tree, source: &[u8], query_str: &str, max_results: usize) -> Result<Vec<QueryResult>> {
    let language = tree.language();
    let query = tree_sitter::Query::new(&language, query_str)?;
    let mut cursor = tree_sitter::QueryCursor::new();
    let matches = cursor.matches(&query, tree.root_node(), source);

    let results = matches
        .take(max_results)
        .map(|m| {
            // For each capture in the match:
            // - capture name
            // - node type
            // - node text
            // - start/end position
        })
        .collect();

    Ok(results)
}
```

### Query result format

```json
[
  {
    "capture": "@function",
    "type": "function_item",
    "text": "fn main() { ... }",
    "start_point": { "row": 10, "column": 0 },
    "end_point": { "row": 15, "column": 1 }
  }
]
```

## Symbol Extraction (`tools/symbols.rs`)

Uses a built-in mapping of node types per language to extract definitions:

```rust
const SYMBOL_TYPES: &[(&str, &[&str])] = &[
    ("function", &["function_item", "function_declaration", "function_definition", "function_signature"]),
    ("class", &["class_declaration", "class_definition", "class_item"]),
    ("struct", &["struct_item", "struct_declaration"]),
    ("enum", &["enum_item", "enum_declaration"]),
    ("interface", &["interface_declaration", "interface_item"]),
    ("import", &["import_statement", "import_declaration", "use_declaration", "use_statement"]),
    ("type", &["type_item", "type_alias_declaration", "type_definition"]),
    ("constant", &["const_item", "const_declaration", "constant_declaration"]),
    ("variable", &["let_declaration", "variable_declaration"]),
];
```

The agent can filter by symbol type via the `symbol_types` parameter.

## Node-at-Position (`tools/node_at.rs`)

Uses tree-sitter's native `descendant_for_point_range()`:

```rust
pub fn node_at_position(tree: &tree_sitter::Tree, source: &[u8], row: usize, column: usize) -> Option<serde_json::Value> {
    let node = tree.root_node().descendant_for_point_range(
        tree_sitter::Point { row, column },
        tree_sitter::Point { row, column },
    );
    Some(node_to_json(node, source, 10))  // generous depth for single node
}
```

## Reference Finding (`tools/references.rs`)

Finds all nodes matching a symbol name. Uses tree-sitter's `node.children_by_field_id` or a walk:

```rust
pub fn find_references(tree: &tree_sitter::Tree, source: &[u8], symbol: &str) -> Vec<serde_json::Value> {
    // Walk all named nodes
    // Check if node.text == symbol and node.is_named()
    // Collect matches with positions
}
```

## Prompt Resource

An MCP prompt that teaches agents to write tree-sitter S-expression queries.

```rust
#[tool_router]
impl TreeSitterServer {
    // ... tools ...
}

impl TreeSitterServer {
    fn query_teaching_prompt() -> String {
        // Markdown content teaching:
        // 1. S-expression syntax basics
        // 2. Node type names per language
        // 3. Common query patterns (find functions, imports, classes)
        // 4. Predicates (#eq?, #match?, #not-eq?)
        // 5. Capture groups (@name)
        // 6. Examples with expected output
    }
}
```

Register as an MCP prompt:

```rust
// In ServerHandler impl
fn get_info(&self) -> ServerInfo {
    ServerInfo {
        // ...
        capabilities: ServerCapabilities::builder()
            .enable_tools()
            .enable_prompts()
            .build(),
        // ...
    }
}
```

## Error Handling

All tool errors return structured `McpError` with descriptive messages:

```rust
impl From<TreeSitterError> for McpError {
    fn from(e: TreeSitterError) -> Self {
        match e {
            TreeSitterError::GrammarNotFound(name) => McpError::invalid_request(
                &format!("Grammar '{}' not found. Run `tree-sitter-mcp grammar install` to provision it.", name)
            ),
            TreeSitterError::ParseError { file, message } => McpError::invalid_request(
                &format!("Failed to parse {}: {}", file, message)
            ),
            TreeSitterError::QueryError(message) => McpError::invalid_request(
                &format!("Invalid query: {}", message)
            ),
            TreeSitterError::FileError(io_err) => McpError::invalid_request(
                &format!("Cannot read file: {}", io_err)
            ),
        }
    }
}
```

## Main Entrypoint

```rust
// main.rs
mod server;
mod config;
mod grammar;
mod tools;
mod prompts;
mod error;

use rmcp::{ServiceExt, transport::stdio};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive(tracing::Level::DEBUG.into())
        )
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    let config = config::load_config()?;
    let server = server::TreeSitterServer::new(config);

    let service = server.serve(stdio()).await.inspect_err(|e| {
        tracing::error!("serving error: {:?}", e);
    })?;

    service.waiting().await?;
    Ok(())
}
```

## Implementation Order

1. **Scaffold**: `main.rs`, `server.rs` with empty tool, verify it compiles and responds to MCP init
2. **Config**: `config.rs` — load TOML, parse extensions map, resolve the server's own grammar dir
3. **Grammar loading**: `grammar/dlopen.rs` + `grammar/mod.rs` — libloading wrapper, GrammarManager
4. **AST serialization**: `tools/ast.rs` — cursor-based node_to_json
5. **parse tool (ast action)**: wire config → grammar → parse → serialize → return
6. **query action**: `tools/query.rs` — compile and run S-expression queries
7. **symbols action**: `tools/symbols.rs` — extract definitions by node type
8. **references action**: `tools/references.rs` — find symbol usages
9. **node_at action**: `tools/node_at.rs` — position lookup
10. **Prompt**: `prompts/mod.rs` — NL → S-expression teaching prompt
11. **Error handling**: `error.rs` — structured error types
12. **Polish**: edge cases, large file handling, testing

### Current grammar-crate test plan

1. Keep `crates/grammar/tests/query_accuracy.rs` split into fixed-case `rstest` snapshots and `quickcheck` invariants.
2. Snapshot structural queries for Rust item names, attributes, and parameters against `fixtures/rust/rust.rs`.
3. Assert malformed structural queries return `GrammarError::Query` without treating empty queries or unknown predicates as compile errors.
4. Keep the `#match?` predicate test ignored until predicate support is implemented in `GrammarEngine::run_query`.
5. Verify with `just test -p grammar` or `cargo test -p grammar` before expanding the grammar tool surface.

## Notes for Implementation

- **Testing**: Use the MCP Inspector (`npx @modelcontextprotocol/inspector ./target/release/tree-sitter-mcp`) to test tool calls
- **Claude Desktop config**: Add to `~/.config/Claude/claude_desktop_config.json`:
  ```json
  {
    "mcpServers": {
      "tree-sitter": {
        "command": "/path/to/target/release/tree-sitter-mcp"
      }
    }
  }
  ```
- **Grammars**: Compiled `.so` grammars live in the server's own grammar directory (`~/.local/share/tree-sitter-mcp/grammars/`). A provisioning CLI (`tree-sitter-mcp grammar install <name>`) fetches and builds grammars into that directory; until then, users drop prebuilt `.so` files there directly.
- **Safety**: The `libloading` dlopen is `unsafe`. The `GrammarLib` wrapper encapsulates the unsafety. The `tree_sitter::Language::from_raw()` call requires the pointer to be valid — this is guaranteed by tree-sitter's ABI contract.
