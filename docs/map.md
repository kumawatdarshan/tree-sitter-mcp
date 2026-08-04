# Map: Tree-Sitter MCP Server

## Destination

A general-purpose, multi-language tree-sitter MCP server in Rust (stdio transport) that dynamically loads language grammars from the server's own grammar directory, exposes semantic navigation tools for code analysis, and provides a prompt teaching agents to translate natural language into tree-sitter S-expression queries. Primary consumer: AI coding agents.

**API Contract:** [`./API.md`](./API.md)

## Notes

- **Rust + rmcp** — the server uses `rmcp` 2.2.0 for MCP protocol handling
- **Own grammar directory** — grammars are compiled shared libraries in the server's own directory (`~/.local/share/tree-sitter-mcp/grammars/`), no third-party editor dependency. A dedicated CLI for fetching and building grammars is planned. Overridable via `--grammar-dir`, `TREE_SITTER_MCP_GRAMMAR_DIR`, or the `grammar_dir` config key.
- **Lazy grammar loading** — grammars are dlopen'd on demand, not at startup. The server starts with zero listed languages; `tree_sitter_get_capabilities` (or any tool that needs a language) triggers the load and caches it for the process lifetime. `GrammarEngine` splits *available* (declared in config) from *loaded* (dlopen'd).
- **No caching** — premature optimization. Parse fresh each request.
- **Named nodes only** — AST output filtered to named nodes, not full leaf nodes
- **18 tools** — 16 semantic navigation tools (phased P0/P1/P2) + 2 building blocks (run_query, find_node)
- **stdio transport** — agent spawns the process, communicates via stdin/stdout
- **File paths on disk** — agent passes file paths, server reads from filesystem
- **Prompt teaches S-expressions** — an MCP prompt resource that maps natural language intent to tree-sitter query syntax
- **Query directory model** — per-language `queries/<language>/*.scm` files with fixed capture vocabulary (see API contract Query Conventions)
- **`tree_sitter_` prefix** — every tool, prompt, and resource carries the `tree_sitter_` wire prefix (`tree_sitter_get_capabilities`, `tree_sitter_run_query`, `tree_sitter_query_guide`, ...)
- **ApiError for tool-domain errors** — distinct from `rmcp::ErrorData` (protocol-level); agents can self-correct

## Decisions so far

<!-- the index — one line per closed ticket -->

- **01 tool-surface-design** → single `parse` tool schema (plan §Tool Surface); superseded by the 18-tool contract (API.md §Tools), phased P0/P1/P2
- **02 grammar-lifecycle** → dlopen compiled grammar libraries via `dlopen2` (`tree_sitter_<name>` constructor derived from filename stem); `GrammarEngine` caches `tree_sitter::Language` per grammar (plan §Grammar Loading)
- **03 nl-to-s-expression-prompt** → MCP prompt resource teaching S-expression queries, shipped as the `query_guide` prompt (plan §Prompt Resource)
- **04 ast-serialization** → named-nodes-only JSON, cursor-based traversal with `max_depth` truncation (plan §AST Serialization)
- **05 mcp-resources** → capabilities and languages as MCP resources (`tree-sitter://capabilities`, `tree-sitter://languages`), not tools (API.md §Capability Negotiation) — **superseded:** capabilities are now negotiated via the `tree_sitter_get_capabilities` **tool** (client supplies languages, server lazy-loads), resources removed
- **06 rust-module-structure** → split crates `app` / `config` / `grammar` / `mcp`; wire types live in the `grammar` crate in P0 (plan §Architecture)
- **07 capabilities-first-negotiation** → `tree_sitter_get_capabilities(languages: Vec<String>)` tool is the session opener: client passes the languages it needs (empty = all available), the server lazy-loads them and returns `Vec<LanguageStatus>` (`Loaded` / `NotConfigured` / `LoadFailed`) so a client can distinguish a typo from a missing grammar. Removes the `tree-sitter://languages` resource. (`grammar` crate, `capabilities.rs`)

## Not yet specified

- How should error messages be shaped for agents? (missing grammar, parse failure, file not found) → **Resolved:** `ApiError` enum with `NotFound`, `Ambiguous`, `CapabilityUnsupported`, `InvalidQuery`, `QueryError`, `Internal` variants (landing in the `grammar` crate)
- What about concurrent requests — does the server handle parallel parses?
- Incremental parsing support — tree-sitter's killer feature, but is it in scope for v1?
- How to handle very large files (megabyte-scale source)?
- How to version or communicate supported languages to the agent? → **Resolved:** the `tree_sitter_get_capabilities` tool is now the discovery surface (empty languages arg returns all configured); the `tree-sitter://languages` resource was removed. `LanguageInfo` still carries `extensions`.

## Out of scope

<!-- ruled beyond the destination -->
