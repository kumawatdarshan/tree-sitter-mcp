# Map: Tree-Sitter MCP Server

## Destination

A general-purpose, multi-language tree-sitter MCP server in Rust (stdio transport) that dynamically loads language grammars from the server's own grammar directory, exposes semantic navigation tools for code analysis, and provides a prompt teaching agents to translate natural language into tree-sitter S-expression queries. Primary consumer: AI coding agents.

**API Contract:** [`./API.md`](./API.md)

## Notes

- **Rust + rmcp** — the server uses `rmcp` 2.2.0 for MCP protocol handling
- **Own grammar directory** — grammars are compiled shared libraries in the server's own directory (`~/.local/share/tree-sitter-mcp/grammars/`), no third-party editor dependency. A dedicated CLI for fetching and building grammars is planned. Overridable via `--grammar-dir`, `TREE_SITTER_MCP_GRAMMAR_DIR`, or the `grammar_dir` config key.
- **No caching** — premature optimization. Parse fresh each request.
- **Named nodes only** — AST output filtered to named nodes, not full leaf nodes
- **18 tools** — 16 semantic navigation tools (phased P0/P1/P2) + 2 building blocks (run_query, find_node)
- **stdio transport** — agent spawns the process, communicates via stdin/stdout
- **File paths on disk** — agent passes file paths, server reads from filesystem
- **Prompt teaches S-expressions** — an MCP prompt resource that maps natural language intent to tree-sitter query syntax
- **Query directory model** — per-language `queries/<language>/*.scm` files with fixed capture vocabulary (see API contract Query Conventions)
- **ApiError for tool-domain errors** — distinct from `rmcp::ErrorData` (protocol-level); agents can self-correct

## Decisions so far

<!-- the index — one line per closed ticket -->

- **01 tool-surface-design** → single `parse` tool schema (plan §Tool Surface); superseded by the 18-tool contract (API.md §Tools), phased P0/P1/P2
- **02 grammar-lifecycle** → dlopen compiled grammar libraries via `dlopen2` (`tree_sitter_<name>` constructor derived from filename stem); `GrammarEngine` caches `tree_sitter::Language` per grammar (plan §Grammar Loading)
- **03 nl-to-s-expression-prompt** → MCP prompt resource teaching S-expression queries, shipped as the `query_guide` prompt (plan §Prompt Resource)
- **04 ast-serialization** → named-nodes-only JSON, cursor-based traversal with `max_depth` truncation (plan §AST Serialization)
- **05 mcp-resources** → capabilities and languages as MCP resources (`tree-sitter://capabilities`, `tree-sitter://languages`), not tools (API.md §Capability Negotiation)
- **06 rust-module-structure** → split crates `app` / `config` / `grammar` / `mcp`; semantic layer lands as a new `semantics` crate in P0 (plan §Architecture)

## Not yet specified

- How should error messages be shaped for agents? (missing grammar, parse failure, file not found) → **Resolved:** `ApiError` enum with `NotFound`, `Ambiguous`, `CapabilityUnsupported`, `InvalidQuery`, `QueryError`, `Internal` variants
- What about concurrent requests — does the server handle parallel parses?
- Incremental parsing support — tree-sitter's killer feature, but is it in scope for v1?
- How to handle very large files (megabyte-scale source)?
- How to version or communicate supported languages to the agent? → **Resolved:** MCP resource at `tree-sitter://capabilities`

## Out of scope

<!-- ruled beyond the destination -->
