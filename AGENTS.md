just test # supports `cargo nextest run` args
just check # supports `cargo check` args
just fmt # treefmt via nix

`docs/` contains the full API contract (`API.md`), implementation plan (`plan.md`), and architecture map (`map.md`). These are the source of truth for the tool surface, query conventions, and phased delivery. Reference these when working on the MCP tools or adding new capabilities.

use Raw string literal instead of escaping where needed. (esp common in tree sitter queries)

## Agent skills

### Issue tracker

Issues live as GitHub issues on `kumawatdarshan/tree-sitter-mcp`; use the `gh` CLI. See `docs/agents/issue-tracker.md`.

### Triage labels

Labels use a scoped vocabulary — the five triage roles map to `review:pending` / `review:inprogress` / `review:agent` / `review:human` / `wontfix`, alongside `area:*`, `priority:*`, and plain `queries`. See `docs/agents/triage-labels.md`.

### Domain docs

Single-context: repo-root `CONTEXT.md` + `docs/adr/`, with the project source of truth in `docs/API.md` / `docs/plan.md` / `docs/map.md`. See `docs/agents/domain.md`.
