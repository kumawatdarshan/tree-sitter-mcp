# Triage Labels

The skills speak in terms of five canonical triage roles. This file maps those roles to the actual label strings used in this repo's issue tracker, and documents the rest of the label vocabulary.

## Triage roles

| Role | Label in our tracker | Meaning |
| -------------------------- | -------------------- | ---------------------------------------- |
| `needs-triage`             | `review:pending`     | New issue awaiting review by a maintainer |
| `needs-info`               | `review:inprogress`  | Review in progress; waiting on the reporter for more information |
| `ready-for-agent`          | `review:agent`       | Fully specified, ready for an AFK agent |
| `ready-for-human`          | `review:human`       | Requires human judgment or implementation |
| `wontfix`                  | `wontfix`            | Will not be actioned (stock GitHub label) |

When a skill mentions a role (e.g. "apply the AFK-ready triage label"), use the corresponding label string from this table.

## Full label vocabulary

Labels use scoped `namespace:value` names where grouping matters, plus a few plain labels. One color per namespace.

### `review:*` — issue lifecycle (the triage roles above)

`review:pending` · `review:inprogress` · `review:agent` · `review:human`

### `area:*` — where the work lands (components + nature of change)

`area:app` (crates/app — entrypoint, telemetry) · `area:config` (crates/config) · `area:grammar` (crates/grammar — engine + wire types) · `area:mcp` (crates/mcp — server, tools, resources, prompts) · `area:docs` · `area:tooling` (nix, just, CI) · `area:api` (wire contract / tool surface) · `area:engine` (core machinery, no wire change) · `area:test` (tests, benches, fixtures) · `area:infra` (build, release, packaging)

### `priority:*`

`priority:high` · `priority:medium` · `priority:low`

### Plain labels

`queries` — the issue touches per-language `.scm` query files (the lang-specific dimension)

### Stock GitHub labels (kept, not part of this vocabulary)

`bug` · `documentation` · `duplicate` · `enhancement` · `good first issue` · `help wanted` · `invalid` · `question` · `wontfix`
