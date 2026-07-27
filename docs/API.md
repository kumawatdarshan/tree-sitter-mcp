# tree-sitter-mcp: API Contract

> A tree-sitter MCP server for AI agents. Structural code navigation and safe
> edit geometry. Replaces grep/find/read loops.
>
> Built against `tree-sitter = "0.26.11"`
> ([docs.rs/tree-sitter/0.26.11](https://docs.rs/tree-sitter/0.26.11/tree_sitter/))
> and `rmcp` (official Rust MCP SDK,
> [docs.rs/rmcp](https://docs.rs/rmcp)).

This contract uses uniform primitives: one locator type, one
pagination envelope, one error type, one sparse-fieldset mechanism.

Language support follows Helix's query-convention model: grammars are
loaded at runtime from Helix's compiled grammar directory (for MVP,
users symlink their Helix runtime; a dedicated CLI for fetching and
building grammars is planned). What's *external* is query
logic only: each language gets a `queries/<language>/*.scm` directory
containing a fixed set of well-known files (`definitions.scm`,
`references.scm`, `calls.scm`, `imports.scm`, `members.scm`,
`scopes.scm`, `edits.scm`), each targeting a **fixed, closed,
cross-language capture vocabulary** that this document defines once,
centrally. A language's capability set is derived by inspecting which
query files exist and compile — not by any self-reported manifest.
This is the same shape as Helix's
`runtime/queries/<language>/highlights.scm` /`locals.scm` convention:
the *files* are per-language data, the *capture names they must use*
are fixed and shared. See **"Query Conventions"** below.

The rmcp error channel distinguishes tool-domain results from
protocol/transport-level failures. `ApiError` is a tool-domain result,
serialized into `CallToolResult.content` with `is_error: Some(true)`.
It is *not* `rmcp::model::ErrorData`, which is reserved for
protocol/transport-level failures (malformed request, unknown tool,
server panic) and is a distinct JSON-RPC-level error path. Conflating
the two means an agent can't tell "your query didn't resolve, try
again" from "the connection is broken."

The contract also includes batched singular tools, corrected defaults,
documented cost tiers, a fields-discovery tool, a pinned wire shape
for batch results, and phased delivery (P0/P1/P2) so agents know
which tools are available at session start.

Two lower-level building-block tools (`run_query` for raw S-expression
escape hatches, `find_node` for position-based node lookup) are
retained alongside the 16 semantic tools — agents use the semantic
tools for normal navigation, and drop down to these when the higher
level doesn't cover a case.

Every tool in this contract is **read-only and side-effect-free** in the
sense that no tool mutates project state or files on disk. Several tools
(`get_relation`, `investigate` at `Extended` radius, `search_symbols` with
`MatchMode::Regex`) are **not O(1)** — see the per-tool cost tier — so
"safe to retry" does not imply "cheap to call in a loop without thought."

---

## Design Principles

1. **One resource model.** Everything is a `Symbol`, addressed by a
   `SymbolLocator`, with sub-resources (`references`, `members`,
   `implementors`, ...) reached via dedicated tools that all share the same
   locator, fieldset, and pagination conventions.
2. **Sparse fieldsets, not flat expand flags.** Callers ask for exactly the
   shape they want, including one level into nested resources, without a
   second round trip. Unknown fields are reported back, never silently
   dropped (see `Fieldset` and `warnings` below).
3. **Uniform pagination.** Every tool returning a list returns a `Page<T>`
   with a continuation cursor. `limit` alone is never sufficient.
4. **Typed errors, correctly routed.** Not-found, ambiguous, and
   capability-unsupported are distinct, structured cases carried as tool
   *content* — never a stringified internal error, and never conflated
   with MCP protocol-level errors.
5. **Discoverability over trial-and-error.** Responses carry hints about
   what else is callable on them, gated by actual per-language capability
   (derived from which query files exist and compile), so a client doesn't
   learn a tool is unsupported by calling it and failing.
6. **Batch as a first-class citizen.** Any tool a client would plausibly
   call once per item in a loop accepts a list of locators instead — this
   now applies uniformly to every singular tool, not just `get_symbols`.
7. **Language logic is data, not code — but the vocabulary it targets is
   fixed.** A `.scm` file is the entire definition of how a language maps
   onto this contract's types. The core ships zero compiled-in knowledge
   of any specific language's grammar shape. It does ship a fixed,
   documented capture-name vocabulary that every language's query files
   are written against, and fixed Rust enums for `SymbolKind` /
   `ReferenceKind` / `MemberKind` / etc. that those capture names resolve
   to.

---

## Core Types

### Position & Range — wrapping `tree_sitter::Point` / `tree_sitter::Range`

`tree_sitter::Point` and `tree_sitter::Range` are used directly wherever a
value is scoped to a single already-known file/tree. Both are
**zero-indexed** and carry no filename — that's the crate's real shape:

```rust
// tree_sitter::Point (docs.rs/tree-sitter/0.26.11/tree_sitter/struct.Point.html)
pub struct Point {
    pub row: usize,     // 0-indexed
    pub column: usize,  // 0-indexed
}

// tree_sitter::Range (docs.rs/tree-sitter/0.26.11/tree_sitter/struct.Range.html)
pub struct Range {
    pub start_byte: usize,
    pub end_byte: usize,
    pub start_point: Point,
    pub end_point: Point,
}
```

Neither type carries a path, and outside a single already-file-scoped
context a range needs to travel with its file:

```rust
/// A tree_sitter::Range qualified with the file it belongs to. Needed
/// anywhere a range crosses tool boundaries without an enclosing Symbol
/// to supply the path.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FileRange {
    pub file: String,
    pub range: SerRange,   // see Serialization note below
}

/// A tree_sitter::Point qualified with its file, for the same reason.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FilePosition {
    pub file: String,
    pub point: SerPoint,
}
```

The bespoke `Range { file, start, end }` is replaced by `FileRange`;
bare positions use `FilePosition`.

### Serialization note

`tree_sitter::Point` and `tree_sitter::Range` do **not** derive
`serde::Serialize` / `Deserialize` — the `tree-sitter` crate has no
`serde` feature to opt into (confirmed against the 0.26.11 crate's
dependency list; `serde_json` appears only as a `build`-time dependency,
unrelated to runtime serialization of these types). This contract's wire
format uses local newtype mirrors:

```rust
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
pub struct SerPoint { pub row: usize, pub column: usize }
impl From<tree_sitter::Point> for SerPoint {
    fn from(p: tree_sitter::Point) -> Self { Self { row: p.row, column: p.column } }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
pub struct SerRange {
    pub start_byte: usize,
    pub end_byte: usize,
    pub start_point: SerPoint,
    pub end_point: SerPoint,
}
impl From<tree_sitter::Range> for SerRange {
    fn from(r: tree_sitter::Range) -> Self {
        Self {
            start_byte: r.start_byte,
            end_byte: r.end_byte,
            start_point: r.start_point.into(),
            end_point: r.end_point.into(),
        }
    }
}
```

The crate types (`tree_sitter::Point`, `tree_sitter::Range`) are used
as-is internally — inside adapters, the query engine, anything that
doesn't cross the MCP wire boundary. `SerPoint` / `SerRange` are the wire
form. `FileRange` / `FilePosition` above use the `Ser*` types directly in
their field definitions.

### SymbolId & SymbolLocator

```rust
/// Structured, not a bare string. Built from the @name captures a
/// language's definitions.scm produces for a symbol and its enclosing
/// symbols, in nesting order. Two symbols are equal iff file, path, and
/// language all match — this is load-bearing for Ambiguous/NotFound
/// resolution below, so it needs real equality semantics, not
/// string-splitting.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct SymbolId {
    pub file: String,
    /// Captured @name segments in nesting order, e.g.
    /// ["QueryParser", "parse_query"] for a method. Anonymous
    /// constructs (closures, unnamed impls) get a synthetic segment
    /// (e.g. "<closure@12:4>") built from position, never an empty
    /// string.
    pub path: Vec<String>,
    pub language: String,
}

/// Every tool that references "a symbol" takes this, not a bare
/// SymbolId. Collapses the old get()/at() split into one addressing
/// scheme.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SymbolLocator {
    Id(SymbolId),
    Position {
        file: String,
        /// 1-indexed, matching editor/LSP line numbers. The server
        /// converts to 0-indexed tree_sitter::Point internally.
        row: usize,
        /// 1-indexed, matching editor/LSP column numbers. Omitted =
        /// innermost symbol spanning the row.
        column: Option<usize>,
    },
}
```

**Disambiguation rule for `SymbolLocator::Position`:** when a position
falls inside nested symbols (e.g. a parameter inside a function inside an
impl block), the **innermost enclosing symbol wins outright** — this
never produces `ApiError::Ambiguous`. `Ambiguous` is reserved for a
future name-based (non-path) locator variant, not reachable by any
locator in this contract today. This is stated explicitly so the variant
isn't unreachable dead code with undefined trigger conditions.

### Fieldset

```rust
/// Dotted-path field selection. "members" pulls the members list at
/// default (compact) shape; "members.doc" additionally pulls each
/// member's doc comment; "context.enclosing.signature" reaches two
/// levels deep.
///
/// Always available at top level regardless of fields requested:
/// id, name, kind, range, language, signature.
///
/// Selectable leaf/branch fields:
///   body | doc | source | complexity
///   members | members.doc | members.body
///   context | context.enclosing | context.parents | context.imports
pub type Fieldset = Vec<String>;
```

Unrecognized field paths are **not silently ignored**. Every
response envelope that accepts a `Fieldset` includes a sibling
`warnings: Vec<String>` populated with any requested field path that
didn't match a known field, so a typo (`"membrs.doc"`) is visible instead
of silently returning an empty result for that branch. See
`describe_fields` below for proactive discovery instead of trial and
error.

### Page (uniform list envelope)

```rust
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub next_cursor: Option<String>,
    /// Best-effort; may be approximate for large result sets.
    pub total_estimate: Option<usize>,
    /// Unrecognized Fieldset paths from the request, if any. Empty
    /// when every requested field matched.
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PageRequest {
    pub limit: Option<usize>,     // per-tool default noted below
    pub cursor: Option<String>,
}
```

### ApiError

This is **not** `rmcp::model::ErrorData`. `ErrorData` is reserved for
protocol/transport-level failures (malformed request, unknown tool name,
server panic) and is returned as a JSON-RPC error per the MCP spec.
`ApiError` is a domain result — every tool returns
`Result<CallToolResult, rmcp::ErrorData>` at the function-signature level
(the shape rmcp's `#[tool]` macro expects), and on the success path,
`ApiError` (when the query itself failed to resolve — not found,
ambiguous, unsupported) is serialized as JSON into
`CallToolResult.content` with `is_error: Some(true)`. This lets an agent
see *why* its query didn't resolve and self-correct, rather than the call
looking like a broken connection. Only genuine protocol/dispatch failures
(the tool name itself doesn't exist, params fail schema validation before
your handler runs) use `rmcp::ErrorData`.

```rust
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ApiError {
    /// Locator didn't resolve. `suggestions` offers near-name matches
    /// so a client can retry intelligently instead of giving up.
    NotFound {
        locator: SymbolLocator,
        suggestions: Vec<SymbolId>,
    },
    /// Reserved for a future name-based locator; unreachable via any
    /// locator variant in this contract today (see disambiguation
    /// rule above).
    Ambiguous {
        locator: SymbolLocator,
        candidates: Vec<SymbolId>,
    },
    /// The target language's query directory has no (or a
    /// non-compiling) query file backing this capability — derived
    /// from file presence, not a self-reported flag.
    CapabilityUnsupported {
        language: String,
        capability: Capability,
    },
    /// Malformed query (bad match_mode/regex, invalid scope, etc.)
    InvalidQuery { reason: String },
    /// A language's .scm query file failed to compile against its
    /// grammar, or is missing a capture this contract requires for
    /// the query type it claims to implement. Distinct from
    /// CapabilityUnsupported (file absent) — this is file present but
    /// broken, a real and routine failure mode for hand-authored
    /// query files, worth surfacing distinctly for whoever maintains
    /// that language's queries.
    QueryError {
        language: String,
        query_type: String,
        reason: String,
    },
    /// Genuine internal failure (IO error, parser crash, etc.)
    Internal { reason: String },
}
```

Not-found is **never** folded into `Option` at the outer layer for plural
tools (empty `Page` is the not-found signal there); for singular tools it
is the explicit `NotFound` variant, not a generic failure.

### Batch result wire shape

Every tool accepting multiple locators (see Design Principle 6) returns
one result per input, in input order, so a batch call's partial failures
don't fail the whole request. The wire shape is pinned explicitly rather
than left to serde's default `Result` representation:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ItemResult<T> {
    Ok { value: T },
    Err { error: ApiError },
}
```

### Symbol

```rust
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Symbol {
    pub id: SymbolId,
    pub name: String,
    pub kind: SymbolKind,
    pub range: FileRange,
    pub language: String,

    // Always included
    pub signature: Signature,

    // Present only if requested via fieldset
    pub body: Option<String>,
    pub doc: Option<String>,
    pub members: Option<Vec<Member>>,
    pub context: Option<Context>,
    pub source: Option<String>,
    pub complexity: Option<Complexity>,
}
```

`available_actions` is **not included on `Symbol`**. Per-symbol
capability truthfulness (does *this specific* struct have implementors
right now) would require an extra query per symbol per page — an
undocumented cost. Capability is a **language-level**
fact derivable once from `ServerCapabilities` at session start (see
below); a client checks it there, not per symbol. If a tool call turns
out to have nothing to return for a fully-supported capability (e.g. a
trait with zero implementors), that's a normal empty `Page`, not a
capability gap.

### Signature / Parameter / Annotation / Visibility

```rust
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Signature {
    pub parameters: Vec<Parameter>,
    pub return_type: Option<String>,
    pub type_parameters: Option<String>,
    pub annotations: Vec<Annotation>,
    pub visibility: Option<Visibility>,
    pub is_async: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Parameter {
    pub name: String,
    pub r#type: Option<String>,
    pub default_value: Option<String>,
    pub is_optional: bool,
    pub is_variadic: bool,
    pub range: FileRange,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Annotation {
    pub name: String,
    pub full_text: String,
    pub range: FileRange,
    pub kind: AnnotationKind,
    pub arguments: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum AnnotationKind {
    Derive,
    Attribute,
    Decorator,
    Macro,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum Visibility {
    Public,
    Private,
    Crate,
    Super,
    Internal,
}
```

### SymbolKind

```rust
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
pub enum SymbolKind {
    Function, Method, Class, Struct, Enum, Trait, Interface, Impl,
    Module, Constant, Static, Macro, TypeAlias, EnumVariant, Field,
    Parameter, Variable, Closure, Test,
}
```

Fixed, closed, compiled-in. This is the payoff of the
Helix-style convention (see **Query Conventions** below): every
language's `definitions.scm` targets capture names like
`@definition.function` / `@definition.struct`, and the core has one fixed
table mapping each recognized capture suffix to a `SymbolKind` variant.
The crate itself has no concept of "symbol kind" — `Node::kind() -> &str`
returns a raw, per-grammar grammar-node-type string (e.g.
`"function_item"` in `tree-sitter-rust`); a query file's capture name is
what normalizes that into this contract's vocabulary, not compiled
per-language Rust code.

### Reference

```rust
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Reference {
    pub range: FileRange,
    pub context: String,
    pub kind: ReferenceKind,
    pub symbol_id: SymbolId,
    pub container_id: SymbolId,
    /// Populated when depth > 1: the chain from the queried symbol
    /// to this reference. Empty for direct (depth-1) references.
    pub path: Vec<SymbolId>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
pub enum ReferenceKind {
    Call, TypeRef, FieldAccess, Import, Override, MacroCall,
    Decoration, Reference,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
pub enum Direction {
    In,   // callers / things that reference this symbol
    Out,  // callees / things this symbol references
    Both,
}
```

### Member / MemberKind

```rust
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Member {
    pub name: String,
    pub kind: MemberKind,
    pub range: FileRange,
    pub signature: Signature,
    // Present only if requested via fieldset (e.g. "members.doc")
    pub doc: Option<String>,
    pub body: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
pub enum MemberKind {
    Field, Method, Variant, AssociatedFn, Constant, Constructor,
}
```

### Context

```rust
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Context {
    pub enclosing: Option<Box<Symbol>>,
    pub parents: Vec<Symbol>,
    pub siblings: Vec<String>,
    pub preview: String,
    pub imports: Vec<ImportInfo>,
}
```

### ImportInfo

```rust
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ImportInfo {
    pub path: String,
    pub symbols: Vec<String>,
    pub aliases: std::collections::HashMap<String, String>,
    pub is_wildcard: bool,
    pub range: FileRange,
}
```

Import resolution (mapping `path` to an actual file, and thereby
resolving cross-file references) is syntactic string-matching against
each language's own import-statement conventions (as declared by
`imports.scm`), not semantic resolution — there is no type checker or
module resolver behind this. Cross-file `references()`/`calls()` results
are therefore only as complete as this string-matching allows; this is a
hard limitation stemming from tree-sitter-only scope (Design Principle 7
/ no LSP), not a bug to be fixed later.

### ModuleTree

```rust
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ModuleTree {
    pub name: String,
    pub path: String,
    pub children: Vec<ModuleTree>,
    pub symbols: Vec<SymbolId>,
}
```

### RelationPath

```rust
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RelationPath {
    pub kind: ReferenceKind,
    /// Ordered list of symbols from `a` to `b` inclusive.
    pub chain: Vec<SymbolId>,
    /// The specific reference edges connecting each hop.
    pub edges: Vec<Reference>,
}
```

### Investigation

```rust
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Investigation {
    pub symbol: Symbol,               // with context expanded one level
    pub immediate_callers: Vec<Reference>,
    pub immediate_callees: Vec<Reference>,
    pub type_refs: Vec<Reference>,
    /// Only populated if the symbol's language + kind support it
    /// (e.g. a trait/interface) — gated by Capability::Implementors
    /// being present for the language, per ServerCapabilities.
    pub implementors: Option<Vec<Symbol>>,
}
```

### EditPlan / EditImpact / Confidence

```rust
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EditPlan {
    pub changes: Vec<EditImpact>,
    pub warnings: Vec<String>,
    /// Opaque token capturing the file version(s) this plan was
    /// computed against. Not checked by any tool in this contract —
    /// there is no apply() tool. Clients apply the byte ranges in
    /// `changes` themselves and are responsible for freshness: this
    /// plan is single-use and must be applied before any other write
    /// touches the affected files, since nothing in this contract
    /// re-validates the ranges against the file's current content.
    pub version_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EditImpact {
    pub file: String,
    /// Bare SerRange, not FileRange, since `file` above already
    /// qualifies it.
    pub range: SerRange,
    pub description: String,
    pub confidence: Confidence,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
pub enum Confidence { High, Medium, Low }
```

### Complexity

```rust
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Complexity {
    pub symbol_id: SymbolId,
    pub cyclomatic: u32,
    pub cognitive: u32,
    pub loc: u32,
    pub max_nesting: u32,
    pub parameter_count: u32,
    pub fan_out: u32,
    pub fan_in: u32,
    pub rating: ComplexityRating,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
pub enum ComplexityRating { Low, Medium, High, VeryHigh }

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ComplexityThresholds {
    pub cyclomatic_medium: u32,    // default 5
    pub cyclomatic_high: u32,      // default 11
    pub cyclomatic_very_high: u32, // default 21
    pub cognitive_medium: u32,     // default 8
    pub cognitive_high: u32,       // default 16
    pub cognitive_very_high: u32,  // default 31
}
```

### Capability / LanguageInfo

```rust
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
pub enum Capability {
    Members,
    References,
    Implementors,
    Modules,
    TypeAlias,
    TestFixtures,
    Complexity,
    EditPlan,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LanguageInfo {
    pub name: String,
    pub extensions: Vec<String>,
    pub capabilities: Vec<Capability>,
}
```

`Capability` stays a closed, compiled enum — it is **not** self-reported
by anything external. A language's `capabilities` list is computed at
server startup by checking, for each `Capability` variant, whether the
corresponding query file exists under `queries/<language>/` **and**
compiles successfully against that language's grammar **and** contains
at least the required capture names for that capability (see Query
Conventions). This is a deterministic, inspectable check — equivalent to
`hx --health <lang>` in Helix — not a value any query file declares about
itself.

`LanguageInfo.name` is this contract's own display string (e.g.
`"rust"`), not derived from `tree_sitter::Language`, which is an opaque
ABI handle with no name of its own (`Language::metadata()` exposes a
grammar's semver when populated, which is useful for diagnostics but
unrelated to this field). Each language has exactly one `tree_sitter::Language`
instance, loaded at runtime from Helix's compiled grammar directory and
registered in a runtime registry — grammars are **not** compiled into
the server binary; only query logic is. `Symbol.language` is this
contract's name string, set by whichever grammar's query set produced
the symbol.

---

## Query Conventions

This section plays the same role for this
project that Helix's fixed highlight-scope vocabulary
(`docs.helix-editor.com/master/themes.html`, "reference for the capture
names used in `highlights.scm`") and its `locals.scm` convention
(`@local.definition` / `@local.scope` / `@local.reference`) play for
Helix: **one central, versioned vocabulary that every language's query
files are written against**, so adding a language means writing `.scm`
files against a known contract, not inventing one.

### Directory layout

```
queries/
├── rust/
│   ├── definitions.scm
│   ├── references.scm
│   ├── calls.scm
│   ├── imports.scm
│   ├── members.scm
│   ├── scopes.scm
│   └── edits.scm
├── typescript/
│   └── ... (same seven files)
└── python/
    └── ... (same seven files)
```

A language directory may omit any file. A missing file means the
corresponding `Capability` is absent for that language — this is how
Python legitimately ends up without `Capability::Implementors` (no
`implementors`-relevant captures possible without a static type checker
backing structural conformance), expressed as "the file doesn't claim
that capture," not as a manifest flag anywhere.

Grammars are **loaded at runtime from Helix's compiled grammar
directory**. At startup, the server discovers the directory (user
symlink for MVP; a dedicated CLI for fetching and building grammars is
planned), loads available `.so` files via dynamic linking, and builds a
runtime registry of `tree_sitter::Language` instances. Only the `.scm`
query text is read from disk and compiled to a `tree_sitter::Query` at
server startup, then cached for the process lifetime (mirroring Helix's
`read_query` + `OnceCell` pattern). A query directory is not a "plugin":
it cannot introduce a new grammar, a new `SymbolKind` variant, or a new
`Capability` — it can only map an existing grammar's node shapes onto the
fixed vocabulary below.

### Fixed capture vocabulary

Every capture name below is **core-recognized**: the server has one
central match table (per query-type file) mapping capture name →
contract type/field. A `.scm` file may use additional captures for its
own internal query logic (e.g. an anonymous helper capture used only to
constrain a `#match?` predicate), but only captures from this table are
read into `Symbol`/`Reference`/`Member`/etc. Unrecognized captures are
inert, exactly as an unrecognized capture in a Helix `highlights.scm`
simply doesn't highlight anything — this is not an error.

#### `definitions.scm`

| Capture | Required? | Populates |
|---|---|---|
| `@definition.function` | one of `@definition.*` required | `SymbolKind::Function` |
| `@definition.method` | | `SymbolKind::Method` |
| `@definition.class` | | `SymbolKind::Class` |
| `@definition.struct` | | `SymbolKind::Struct` |
| `@definition.enum` | | `SymbolKind::Enum` |
| `@definition.trait` | | `SymbolKind::Trait` |
| `@definition.interface` | | `SymbolKind::Interface` |
| `@definition.impl` | | `SymbolKind::Impl` |
| `@definition.module` | | `SymbolKind::Module` |
| `@definition.constant` | | `SymbolKind::Constant` |
| `@definition.static` | | `SymbolKind::Static` |
| `@definition.macro` | | `SymbolKind::Macro` |
| `@definition.type_alias` | | `SymbolKind::TypeAlias` |
| `@definition.enum_variant` | | `SymbolKind::EnumVariant` |
| `@definition.field` | | `SymbolKind::Field` |
| `@definition.closure` | | `SymbolKind::Closure` |
| `@definition.test` | | `SymbolKind::Test` (in addition to whichever `@definition.*` also matches — a symbol can be both `Function` and `Test`; `Test` is applied as a tag via a second, overlapping capture, not an exclusive kind) |
| `@name` | **required** alongside every `@definition.*` | `Symbol.name`, and one segment of `SymbolId.path` |
| `@signature` | recommended | bounds of `Symbol.signature`'s source span |
| `@body` | required for `Capability` body-expansion to work | `Symbol.body` when requested via fieldset |
| `@doc` | optional | `Symbol.doc` when requested via fieldset |
| `@annotation.derive` / `@annotation.attribute` / `@annotation.decorator` / `@annotation.macro` | optional | `Annotation.kind` variants |
| `@parameter` | optional, inside a `@signature` span | `Signature.parameters` entries |
| `@visibility.public` / `@visibility.private` / `@visibility.crate` / `@visibility.super` / `@visibility.internal` | optional | `Signature.visibility` |

#### `references.scm`

| Capture | Required? | Populates |
|---|---|---|
| `@reference.call` | one of `@reference.*` required for `Capability::References` | `ReferenceKind::Call` |
| `@reference.type_ref` | | `ReferenceKind::TypeRef` |
| `@reference.field_access` | | `ReferenceKind::FieldAccess` |
| `@reference.import` | | `ReferenceKind::Import` |
| `@reference.override` | | `ReferenceKind::Override` |
| `@reference.macro_call` | | `ReferenceKind::MacroCall` |
| `@reference.decoration` | | `ReferenceKind::Decoration` |
| `@reference.generic` | | `ReferenceKind::Reference` (fallback kind) |

#### `calls.scm`

| Capture | Required? | Populates |
|---|---|---|
| `@caller` | both required for `Capability::References`'s call-direction queries | `Reference.container_id` side of a call edge |
| `@callee` | | `Reference.symbol_id` side of a call edge |

#### `imports.scm`

| Capture | Required? | Populates |
|---|---|---|
| `@import_path` | required | `ImportInfo.path` |
| `@import_symbol` | optional | entries in `ImportInfo.symbols` |
| `@import_alias` | optional, paired with `@import_symbol` | a key/value in `ImportInfo.aliases` |
| `@import_wildcard` | optional | sets `ImportInfo.is_wildcard = true` |

#### `members.scm`

| Capture | Required? | Populates |
|---|---|---|
| `@member.field` | one of `@member.*` required for `Capability::Members` | `MemberKind::Field` |
| `@member.method` | | `MemberKind::Method` |
| `@member.variant` | | `MemberKind::Variant` |
| `@member.associated_fn` | | `MemberKind::AssociatedFn` |
| `@member.constant` | | `MemberKind::Constant` |
| `@member.constructor` | | `MemberKind::Constructor` |

#### `scopes.scm`

Modeled directly on Helix's `locals.scm` convention — same three-capture
shape, repurposed from "highlight persistence" to "reference resolution
scope":

| Capture | Required? | Populates |
|---|---|---|
| `@local.scope` | required for any scope-aware query | bounds used to resolve which `@local.definition` a `@local.reference` binds to |
| `@local.definition` | required | binds a name within its enclosing `@local.scope` |
| `@local.reference` | required | a use-site resolved against the nearest enclosing `@local.definition` of the same name |

#### `edits.scm`

| Capture | Required? | Populates |
|---|---|---|
| `@edit.full` | required for `Capability::EditPlan` | `RangeTarget::Full` bounds |
| `@edit.signature` | required | `RangeTarget::Signature` bounds — explicitly excludes annotation/doc spans even if `@annotation`/`@doc` overlap |
| `@edit.body` | required | `RangeTarget::Body` bounds |
| `@edit.name` | required | `RangeTarget::Name` bounds |

### Capability derivation table

| `Capability` | Present iff |
|---|---|
| `Members` | `members.scm` exists, compiles, and contains ≥1 `@member.*` capture |
| `References` | `references.scm` exists, compiles, and contains ≥1 `@reference.*` capture |
| `Implementors` | `references.scm` contains `@reference.type_ref` or `@reference.override` usable to detect trait/interface conformance **and** the language's `definitions.scm` has `@definition.trait` or `@definition.interface` — both sides required, which is why Python (no trait/interface construct) never has this capability regardless of what `references.scm` contains |
| `Modules` | `definitions.scm` contains `@definition.module` |
| `TypeAlias` | `definitions.scm` contains `@definition.type_alias` |
| `TestFixtures` | `definitions.scm` contains `@definition.test` **and** `scopes.scm` exists (fixture resolution needs scope data) |
| `Complexity` | computed from the parse tree directly (branching node kinds, nesting depth) — needs no query file at all, always available once a grammar is registered |
| `EditPlan` | `edits.scm` exists and compiles with all four required captures present |

### `describe_fields`

```rust
pub async fn describe_fields(
    kind: Option<SymbolKind>,
) -> Result<Vec<FieldDescriptor>, ApiError>
```

```rust
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FieldDescriptor {
    pub path: String,          // e.g. "members.doc"
    pub description: String,
    /// Whether this path is populated for the given `kind` filter (or
    /// for any kind, if `kind` was None) — lets a client check before
    /// spending a real query on a fieldset that would come back empty
    /// for this symbol's kind (e.g. "members" is meaningless for a
    /// SymbolKind::Function).
    pub applicable: bool,
}
```

---

## Tools (18)

Every tool below accepts fieldsets and locators using the shared types
above. Parameter names are consistent across tools: `fields` (not
`expand`), `page` (not bare `limit`), `locator`/`locators` (not `id`/`file`
+ `line`). **Cost tier** is noted per tool — `O(1)`-ish lookups vs.
page-bounded scans vs. genuine graph traversal, so "safe to call" isn't
mistaken for "free to call."

Tools are delivered in phases:
- **P0** (v1): semantic navigation tools available at launch
- **P1** (next): additional semantic tools after P0 is proven
- **P2** (future): complex analysis and edit-planning tools

Two lower-level building-block tools (`run_query`, `find_node`) are
always available — they provide raw tree-sitter access as an escape
hatch when the semantic tools don't cover a case.

### 1. `get_symbols` — **P0**

**Cost: O(batch size), each item O(1) lookup.**
Fetches one or more symbols by
ID or by position.

```rust
pub async fn get_symbols(
    locators: Vec<SymbolLocator>,
    fields: Option<Fieldset>,
) -> Result<Vec<ItemResult<Symbol>>, ApiError>
```

---

### 2. `list_symbols` — **P0**

**Cost: O(page size).**
List symbols in files or directories.

```rust
pub async fn list_symbols(
    paths: Vec<String>,
    kinds: Option<Vec<SymbolKind>>,
    fields: Option<Fieldset>,
    page: Option<PageRequest>,   // default limit 50
) -> Result<Page<Symbol>, ApiError>
```

---

### 3. `search_symbols` — **P0**

**Cost: O(project size) for `Exact`/`Prefix`/`Fuzzy` against a cached
name index; `Regex` mode is O(project size × pattern cost) — see note
below.**
Find symbols by name.

```rust
pub async fn search_symbols(
    query: String,
    match_mode: Option<MatchMode>,   // default Fuzzy
    kind: Option<Vec<SymbolKind>>,
    scope: Option<Scope>,            // required (no default) when match_mode is Regex
    fields: Option<Fieldset>,
    page: Option<PageRequest>,       // default limit 20
) -> Result<Page<ScoredSymbol>, ApiError>
```

```rust
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
pub enum MatchMode { Exact, Prefix, Fuzzy, Regex }

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Scope {
    File(String),
    Directory(String),
    Within(SymbolId),
    Module(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ScoredSymbol {
    pub symbol: Symbol,
    /// Ranking signal only — not comparable across queries or match
    /// modes, and not guaranteed to follow any named algorithm.
    /// 1.0 always means exact/best match within this result set;
    /// use only for relative ordering within one response.
    pub score: f32,
}
```

`Regex` mode uses Rust's `regex` crate, which guarantees linear-time
matching (no catastrophic backtracking) — stated explicitly so a client
doesn't have to assume worst-case ReDoS risk. `scope` becomes a required
parameter (not optional) when `match_mode: Regex` is set; omitting it in
that case returns `ApiError::InvalidQuery`, since an unscoped regex scan
of an entire project is the one operation in this tool whose cost isn't
bounded by a name-index lookup.

Results are ordered by `score` descending, then by file path, then by
range start.

---

### 4. `get_symbol_members` — **P0**

**Cost: O(page size).** Requires `Capability::Members`.

```rust
pub async fn get_symbol_members(
    locators: Vec<SymbolLocator>,
    kinds: Option<Vec<MemberKind>>,
    fields: Option<Fieldset>,       // e.g. ["doc"], ["body"]
    page: Option<PageRequest>,      // default limit 50
) -> Result<Vec<ItemResult<Page<Member>>>, ApiError>
```

---

### 5. `get_symbol_references` — **P0**

**Cost: O(page size) at `depth: 1`; grows with fan-out at `depth > 1` —
bounded by `page`, with `total_estimate` signaling truncation.**
`direction` and
`depth` apply uniformly to any `ReferenceKind`, not just `Call`.

```rust
pub async fn get_symbol_references(
    locators: Vec<SymbolLocator>,
    kinds: Option<Vec<ReferenceKind>>,   // default: all kinds
    direction: Direction,                // required — no default; Both is the expensive case and should be opted into
    depth: Option<usize>,                // default 1; >1 walks transitively
    exclude_definitions: Option<bool>,   // default false
    page: Option<PageRequest>,           // default limit 50
) -> Result<Vec<ItemResult<Page<Reference>>>, ApiError>
```

For `depth > 1`, each `Reference.path` carries the intermediate chain.

---

### 6. `get_symbol_imports` — **P1**

**Cost: O(page size).**

```rust
pub async fn get_symbol_imports(
    paths: Vec<String>,
    page: Option<PageRequest>,   // default limit 100
) -> Result<Page<ImportInfo>, ApiError>
```

---

### 7. `get_symbol_range` — **P0**

**Cost: O(batch size), each item O(1).**

```rust
pub async fn get_symbol_range(
    locators: Vec<SymbolLocator>,
    what: Option<RangeTarget>,           // default Full
    include_annotations: Option<bool>,   // default true
    include_doc: Option<bool>,           // default true
) -> Result<Vec<ItemResult<FileRange>>, ApiError>
```

```rust
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
pub enum RangeTarget {
    /// The whole symbol including annotations/doc per flags above.
    Full,
    /// Header only: visibility + name + generics + parameter list +
    /// return type. Explicitly does NOT include annotations/doc
    /// regardless of the include_* flags; those flags only affect
    /// Full and Body.
    Signature,
    /// The `{ ... }` block (or language equivalent) only.
    Body,
    /// Just the identifier token.
    Name,
}
```

---

### 8. `plan_edit` — **P2**

**Cost: O(affected symbol's reference count) — this is a full
`get_symbol_references` scan under the hood for `Rename`/`Delete`.**
Dry-run an edit operation. Read-only: computes impact, does not write.
No `apply()` tool exists in this contract — see `EditPlan.version_token`
staleness note above for what that means for clients.

```rust
pub async fn plan_edit(
    operation: EditOperation,
    target: SymbolLocator,
    params: Option<EditParams>,
) -> Result<EditPlan, ApiError>
```

```rust
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
pub enum EditOperation { Rename, Move, Extract, Delete }

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EditParams {
    pub new_name: Option<String>,
    pub destination: Option<String>,
    pub range: Option<FileRange>,
}
```

---

### 9. `get_relation` — **P2**

**Cost: O(bounded graph search) — genuinely non-trivial. Default
`max_depth` is 3.**
Direct connectivity query between two symbols, avoiding a full
reference/implementor dump when the question is just "how are these two
connected."

```rust
pub async fn get_relation(
    a: SymbolLocator,
    b: SymbolLocator,
    kinds: Option<Vec<ReferenceKind>>,   // default: all kinds
    max_depth: Option<usize>,            // default 3
) -> Result<Option<RelationPath>, ApiError>
```

Returns `Ok(None)` (not an error) if no relation within `max_depth` is
found.

---

### 10. `investigate` — **P2**

**Cost: `Immediate` radius ≈ 3–4 bounded lookups; `Extended` radius adds
a depth-2 fan-out scan — noticeably more expensive, opt in deliberately.**
Composite "orient me" tool — the single call that answers "I'm looking
at this file:line (e.g. an error site), what do I need to know."

```rust
pub async fn investigate(
    locator: SymbolLocator,
    radius: Option<InvestigateRadius>,   // default Immediate
) -> Result<Investigation, ApiError>
```

```rust
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
pub enum InvestigateRadius {
    /// Symbol + context + direct (depth-1) callers/callees/type-refs.
    Immediate,
    /// Same, with depth-2 callers/callees included.
    Extended,
}
```

---

### 11. `get_symbol_implementors` — **P1**

**Cost: O(page size) against a precomputed implementor index.** Requires
`Capability::Implementors`.

```rust
pub async fn get_symbol_implementors(
    locators: Vec<SymbolLocator>,
    fields: Option<Fieldset>,
    page: Option<PageRequest>,   // default limit 20
) -> Result<Vec<ItemResult<Page<Symbol>>>, ApiError>
```

---

### 12. `get_modules` — **P1**

**Cost: O(module tree size within `depth`).** Requires
`Capability::Modules`.

```rust
pub async fn get_modules(
    scope: Option<Scope>,        // default project root
    depth: Option<usize>,        // default unlimited
) -> Result<ModuleTree, ApiError>
```

---

### 13. `get_type_alias_value` — **P1**

**Cost: O(batch size), each item O(1).** Requires `Capability::TypeAlias`.

```rust
pub async fn get_type_alias_value(
    locators: Vec<SymbolLocator>,
) -> Result<Vec<ItemResult<Option<String>>>, ApiError>
```

`Ok(ItemResult::Ok { value: None })` means "this symbol exists and was
resolved, but is not a type alias" — distinct from
`ItemResult::Err { error: NotFound }`, which means the locator didn't
resolve at all.

---

### 14. `get_test_fixtures` — **P1**

**Cost: O(page size).** Requires `Capability::TestFixtures`.

```rust
pub async fn get_test_fixtures(
    locators: Vec<SymbolLocator>,
    page: Option<PageRequest>,
) -> Result<Vec<ItemResult<Page<Symbol>>>, ApiError>
```

---

### 15. `get_complexity` — **P2**

**Cost: O(batch size), each item O(symbol's own body size) — always
available; needs no query file (see Capability derivation table).**
Also reachable inline via `fields: ["complexity"]` on any
symbol-returning tool — both paths return the identical `Complexity`
shape.

```rust
pub async fn get_complexity(
    locators: Vec<SymbolLocator>,
    metrics: Option<Vec<ComplexityMetric>>,   // default: all
    thresholds: Option<ComplexityThresholds>, // default: struct defaults
) -> Result<Vec<ItemResult<Complexity>>, ApiError>
```

```rust
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
pub enum ComplexityMetric {
    Cyclomatic, Cognitive, Loc, Nesting, Parameters, FanIn, FanOut,
}
```

---

### 16. `describe_fields` — **P0**

**Cost: O(1), static table lookup.** See Query Conventions
above for rationale.

```rust
pub async fn describe_fields(
    kind: Option<SymbolKind>,
) -> Result<Vec<FieldDescriptor>, ApiError>
```

---

### 17. `run_query` — **Building Block** (always available)

**Cost: O(tree size × query cost).** Raw S-expression query escape
hatch. Agents use the semantic tools for normal navigation; drop down
to this when the higher level doesn't cover a case. Read-only; does
not modify the file.

```rust
pub async fn run_query(
    path: String,
    language: Option<String>,
    query: String,
    range: Option<FileRange>,
) -> Result<Vec<QueryMatch>, ApiError>
```

### 18. `find_node` — **Building Block** (always available)

**Cost: O(tree depth).** Position-based node lookup — finds the
smallest named node at a byte offset and returns its ancestor chain.
Maps to `SymbolLocator::Position` internally. Read-only; does not
modify the file.

```rust
pub async fn find_node(
    path: String,
    language: Option<String>,
    byte: usize,
) -> Result<FindNodeResult, ApiError>
```

---

## Capability Negotiation (MCP Resource)

Capabilities are surfaced as an MCP resource, not a tool — the data is
static after server startup (computed once by walking each language's
`queries/<language>/` directory per the Capability derivation table above)
and doesn't change during a session.

```rust
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ServerCapabilities {
    pub languages: Vec<LanguageInfo>,
    pub tools: Vec<ToolInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ToolInfo {
    pub name: String,
    pub description: String,
    /// Generated via #[derive(JsonSchema)] on each tool's params
    /// struct (rmcp + schemars), not hand-written JSON.
    pub input_schema: serde_json::Value,
}
```

Resource URI: `tree-sitter://capabilities`

Example resource content — capabilities are computed once at server startup
by walking each language's `queries/<language>/` directory per the
Capability derivation table above, not self-reported by anything:

```json
{
  "languages": [
    {
      "name": "rust",
      "extensions": [".rs"],
      "capabilities": ["members", "references", "implementors", "modules", "type_alias", "complexity", "edit_plan"]
    },
    {
      "name": "typescript",
      "extensions": [".ts", ".tsx"],
      "capabilities": ["members", "references", "implementors", "modules", "type_alias", "complexity", "edit_plan"]
    },
    {
      "name": "python",
      "extensions": [".py"],
      "capabilities": ["members", "references", "modules", "type_alias", "test_fixtures", "complexity"]
    }
  ]
}
```

A client should read this resource once at session start. This is the
**only** place capability information is surfaced — there is no
per-symbol echo of it.

---

## Summary

| # | Tool | Phase | Notes |
|---|---|---|---|
| 1 | `get_symbols` | P0 | batched, unified locator |
| 2 | `list_symbols` | P0 | paginated |
| 3 | `search_symbols` | P0 | typed match_mode, scored, typed scope, `Regex` requires `scope` |
| 4 | `get_symbol_members` | P0 | batched, fieldset-aware |
| 5 | `get_symbol_references` | P0 | batched, direction+depth generalized, `direction` required |
| 6 | `get_symbol_imports` | P1 | paginated |
| 7 | `get_symbol_range` | P0 | batched, Signature target disambiguated |
| 8 | `plan_edit` | P2 | carries version_token; staleness policy documented |
| 9 | `get_relation` | P2 | `max_depth` default 3 |
| 10 | `investigate` | P2 | composite; cost tier documented per radius |
| 11 | `get_symbol_implementors` | P1 | batched, capability-gated |
| 12 | `get_modules` | P1 | capability-gated |
| 13 | `get_type_alias_value` | P1 | batched, Option vs NotFound disambiguated |
| 14 | `get_test_fixtures` | P1 | batched, capability-gated |
| 15 | `get_complexity` | P2 | batched, gated, thresholds wired in |
| 16 | `describe_fields` | P0 | static field discovery |
| 17 | `run_query` | BB | raw S-expression escape hatch (always available) |
| 18 | `find_node` | BB | position-based node lookup (always available) |

**Total: 16 semantic tools (P0: 7, P1: 4, P2: 5) + 2 building blocks.**

---

## Mapping to `tree-sitter` and `rmcp`

### `tree-sitter = "0.26.11"`

| Contract type | Backed by / derived from `tree_sitter::…` | Notes |
|---|---|---|
| `FileRange` | `Range` + a `file: String`, via `SerRange` | Crate's `Range` carries no filename and isn't serializable; wrapped for both reasons. |
| `FilePosition` | `Point` + a `file: String`, via `SerPoint` | Same reasoning. |
| `SymbolLocator::Position { row, column }` | `Point { row, column }` | 1-indexed on the wire (matching editor/LSP convention); converted to 0-indexed `tree_sitter::Point` internally. |
| `SymbolKind` | *(no crate equivalent)* — populated via the fixed capture vocabulary above, matched against `Node::kind()` / `Node::kind_id()` inside each `.scm` file's own pattern matching | The crate has no symbol-kind concept; this contract's Query Conventions section is the entire translation layer, and it is centrally defined, not per-language Rust code. |
| `Symbol.language: String` | *(no crate equivalent)* — each language has one `tree_sitter::Language` loaded at runtime from Helix's compiled grammar directory | `Language` is an opaque, non-serializable ABI handle; never sent over the wire. |
| `LanguageInfo.name` | *(no crate equivalent)* | `Language::metadata()` gives grammar semver, not a name; unused here. |
| `EditPlan.version_token` | *(conceptually adjacent to)* `Tree` + `InputEdit` | A real `apply()` (out of scope) would use `Tree::edit(&InputEdit)` for incremental re-parse; `version_token` is a placeholder with an explicit staleness caveat, not a checked value. |
| `Signature`, `Member`, `Reference`, `Complexity`, `Page`, `ApiError`, `Capability`, `RelationPath`, `Investigation` | *(no crate equivalent)* | This project's own semantic layer on top of syntax trees and query results. |

Types used as-is, server-internal only (never cross the wire — see
Serialization note): `tree_sitter::Parser`, `Tree`, `Node`, `TreeCursor`,
`Query`, `QueryCursor`, `QueryMatch`, `QueryCapture`, `InputEdit`.

### `rmcp`

| Contract concept | Backed by / derived from `rmcp::…` | Notes |
|---|---|---|
| Tool dispatch | `#[tool_router]` / `#[tool]` / `#[tool_handler]` macros, `ToolRouter<Self>` | Generates the `call_tool` match-arm boilerplate; carries no semantic knowledge of this contract's types. |
| `ToolInfo.input_schema` | `schemars::JsonSchema` derive on each tool's params struct | Free correctness: every `#[derive(..., JsonSchema)]` struct in this document (all of them) generates its own schema — no hand-written JSON needed anywhere in this contract. |
| Transport-level failure | `rmcp::model::ErrorData` (aka `ErrorCode` + message), returned as `Result<CallToolResult, ErrorData>` at each tool function's signature | Reserved for protocol/dispatch failures only (unknown tool, malformed params that fail schema validation before the handler body runs, server panic). **Never** used to carry `ApiError`. |
| Tool-domain failure | `ApiError`, serialized into `CallToolResult { content, is_error: Some(true), .. }` | Per the MCP spec: "any errors that originate from the tool SHOULD be reported inside the result object... Otherwise, the LLM would not be able to see that an error occurred and self-correct." This is why `ApiError` is content, not `ErrorData`. |
| Capability negotiation | MCP resource (`tree-sitter://capabilities`) | Static data after startup; read via `read_resource`, not a tool call. `LanguageInfo` with computed capabilities per language. |

`rmcp` provides **zero** of this contract's semantic types (`SymbolId`,
`Fieldset`, `Page`, `ApiError`'s variants, etc.) — same as `tree-sitter`,
it gives protocol plumbing and a schema-derivation macro, not domain
modeling. Everything above the dispatch/schema layer is this project's
own design.
