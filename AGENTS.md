# tree-sitter-mcp

Rust MCP server that exposes tree-sitter parsing and querying over stdin/stdout.

## Quick commands

```sh
just test # supports `cargo nextest run` args
just check # supports `cargo check` args
just fmt # treefmt via nix
```

## Design docs

`.scratch/tree-sitter-mcp-server/` contains the full API contract (`API.md`), implementation plan (`plan.md`), and architecture map (`map.md`). These are the source of truth for the tool surface, query conventions, and phased delivery. Reference these when working on the MCP tools or adding new capabilities.
