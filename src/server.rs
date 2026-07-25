use rmcp::{
    ErrorData, RoleServer, ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{
        CallToolResult, ContentBlock, ErrorCode, GetPromptResult, Implementation,
        ListResourceTemplatesResult, ListResourcesResult, PaginatedRequestParams, PromptMessage,
        ReadResourceRequestParams, ReadResourceResult, Resource, ResourceContents,
        ResourceTemplate, Role, ServerCapabilities,
    },
    prompt, prompt_handler, prompt_router, schemars,
    service::RequestContext,
    tool, tool_handler, tool_router,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::Path, sync::Arc};
use tree_sitter::{Node, Parser, Query, QueryCursor, StreamingIterator, Tree};

/// One entry per supported language: its tree-sitter `Language` handle and
/// the file extensions that infer it.
// TODO: we need to integrate this with @config/extensions.rs
pub struct LanguageEntry {
    pub language: tree_sitter::Language,
    pub extensions: &'static [&'static str],
}

// TODO: replace with our config infrastructure
// fn build_registry() -> HashMap<&'static str, LanguageEntry> {
//     let mut map = HashMap::new();

//     map.insert(
//         "rust",
//         LanguageEntry {
//             language: tree_sitter_rust::LANGUAGE.into(),
//             extensions: &["rs"],
//         },
//     );
//     map.insert(
//         "python",
//         LanguageEntry {
//             language: tree_sitter_python::LANGUAGE.into(),
//             extensions: &["py", "pyi"],
//         },
//     );
//     map.insert(
//         "typescript",
//         LanguageEntry {
//             language: tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
//             extensions: &["ts", "mts", "cts"],
//         },
//     );
//     map.insert(
//         "tsx",
//         LanguageEntry {
//             language: tree_sitter_typescript::LANGUAGE_TSX.into(),
//             extensions: &["tsx"],
//         },
//     );
//     map.insert(
//         "javascript",
//         LanguageEntry {
//             language: tree_sitter_javascript::LANGUAGE.into(),
//             extensions: &["js", "mjs", "cjs", "jsx"],
//         },
//     );

//     map
// }

/// A `[start, end)` byte range within the source file.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ByteRange {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct DumpAstParams {
    #[schemars(description = "Absolute or workspace-relative path to the source file")]
    pub path: String,

    #[schemars(
        description = "Language id (e.g. 'rust', 'python', 'typescript', 'tsx', 'javascript'). Inferred from the file extension if omitted."
    )]
    pub language: Option<String>,

    #[schemars(
        description = "Restrict the dump to the smallest node covering this byte range, instead of the whole file"
    )]
    pub range: Option<ByteRange>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct RunQueryParams {
    #[schemars(description = "Absolute or workspace-relative path to the source file")]
    pub path: String,

    #[schemars(description = "Language id. Inferred from the file extension if omitted.")]
    pub language: Option<String>,

    #[schemars(
        description = "A tree-sitter S-expression query, e.g. \"(function_item name: (identifier) @name)\". Use the query_guide prompt for syntax help."
    )]
    pub query: String,

    #[schemars(description = "Restrict the search to this byte range instead of the whole file")]
    pub range: Option<ByteRange>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct FindNodeParams {
    #[schemars(description = "Absolute or workspace-relative path to the source file")]
    pub path: String,

    #[schemars(description = "Language id. Inferred from the file extension if omitted.")]
    pub language: Option<String>,

    #[schemars(description = "Byte offset into the file to locate")]
    pub byte: usize,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct QueryMatch {
    pub pattern_index: usize,
    pub captures: Vec<Capture>,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct Capture {
    pub name: String,
    pub start_byte: usize,
    pub end_byte: usize,
    pub start_point: (usize, usize),
    pub end_point: (usize, usize),
    pub text: String,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct NodeInfo {
    pub kind: String,
    pub start_byte: usize,
    pub end_byte: usize,
    pub start_point: (usize, usize),
    pub end_point: (usize, usize),
    pub text: String,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct FindNodeResult {
    /// Innermost node first, root last.
    pub ancestors: Vec<NodeInfo>,
}

// TODO: not sure if this is how it should be handled. considering we are loading the languages from a runtime directory
#[derive(Clone)]
pub struct TreeSitterServer {
    languages: Arc<HashMap<&'static str, LanguageEntry>>,
    tool_router: ToolRouter<Self>,
}

impl Default for TreeSitterServer {
    fn default() -> Self {
        Self::new()
    }
}

impl TreeSitterServer {
    pub fn new() -> Self {
        Self {
            languages: Arc::new(build_registry()),
            tool_router: Self::tool_router(),
        }
    }

    /// Resolve a language id, or infer one from the file extension.
    /// Error messages list valid ids and the inference fallback, per the
    /// "actionable error messages" best practice.
    fn resolve_language(
        &self,
        path: &str,
        requested: Option<&str>,
    ) -> Result<&LanguageEntry, ErrorData> {
        if let Some(id) = requested {
            return self.languages.get(id).ok_or_else(|| {
                ErrorData::new(
                    ErrorCode::INVALID_PARAMS,
                    format!(
                        "unknown language '{id}'. Supported languages: {:?}. \
                         Omit `language` to infer it from the file extension instead.",
                        self.sorted_language_ids()
                    ),
                    None,
                )
            });
        }
        let ext = Path::new(path).extension().and_then(|e| e.to_str());
        self.languages
            .values()
            .find(|entry| ext.is_some_and(|e| entry.extensions.contains(&e)))
            .ok_or_else(|| {
                ErrorData::new(
                    ErrorCode::INVALID_PARAMS,
                    format!(
                        "could not infer a language for '{path}' (extension: {:?}). \
                         Pass `language` explicitly. Supported languages: {:?}.",
                        ext,
                        self.sorted_language_ids()
                    ),
                    None,
                )
            })
    }

    fn sorted_language_ids(&self) -> Vec<&'static str> {
        let mut ids: Vec<_> = self.languages.keys().copied().collect();
        ids.sort_unstable();
        ids
    }

    /// Read the file, resolve its language, and parse it. Shared by all
    /// three parsing tools so none of them duplicate this boilerplate.
    fn load_tree(&self, path: &str, language: Option<&str>) -> Result<(String, Tree), ErrorData> {
        let entry = self.resolve_language(path, language)?;

        let source = std::fs::read_to_string(path).map_err(|e| {
            ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                format!(
                    "could not read '{path}': {e}. Check that the path exists \
                     and is readable from the server's working directory."
                ),
                None,
            )
        })?;

        let mut parser = Parser::new();
        parser.set_language(&entry.language).map_err(|e| {
            ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                format!("failed to load grammar: {e}"),
                None,
            )
        })?;

        let tree = parser.parse(&source, None).ok_or_else(|| {
            ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                "tree-sitter returned no tree for this file (possibly cancelled or too large)"
                    .to_string(),
                None,
            )
        })?;

        Ok((source, tree))
    }

    /// Narrow a root node to the smallest node covering a byte range, if given.
    fn apply_range<'a>(root: Node<'a>, range: Option<&ByteRange>) -> Node<'a> {
        match range {
            Some(r) => root
                .descendant_for_byte_range(r.start, r.end)
                .unwrap_or(root),
            None => root,
        }
    }

    fn node_info(node: Node<'_>, source: &str) -> NodeInfo {
        NodeInfo {
            kind: node.kind().to_string(),
            start_byte: node.start_byte(),
            end_byte: node.end_byte(),
            start_point: (node.start_position().row, node.start_position().column),
            end_point: (node.end_position().row, node.end_position().column),
            text: node
                .utf8_text(source.as_bytes())
                .unwrap_or("<invalid utf8>")
                .to_string(),
        }
    }
}

#[tool_router]
impl TreeSitterServer {
    #[tool(
        description = "Ping the server to verify connectivity",
        annotations(
            title = "Ping",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn ping(&self) -> String {
        "pong".into()
    }

    #[tool(
        description = "Dump the tree-sitter S-expression AST for a source file. \
                        Optionally restrict the dump to the smallest node covering a byte range. \
                        Read-only; does not modify the file.",
        annotations(
            title = "Dump AST",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn tree_sitter_dump_ast(
        &self,
        Parameters(params): Parameters<DumpAstParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let (_source, tree) = self.load_tree(&params.path, params.language.as_deref())?;
        let root = Self::apply_range(tree.root_node(), params.range.as_ref());

        Ok(CallToolResult::success(vec![ContentBlock::text(
            root.to_sexp(),
        )]))
    }

    #[tool(
        description = "Run a tree-sitter S-expression query against a source file and return \
                        matches with captured node names, byte/point ranges, and text. \
                        Read-only; does not modify the file. Use the query_guide prompt for query syntax.",
        annotations(
            title = "Run Query",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn tree_sitter_run_query(
        &self,
        Parameters(params): Parameters<RunQueryParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let entry = self.resolve_language(&params.path, params.language.as_deref())?;
        let (source, tree) = self.load_tree(&params.path, params.language.as_deref())?;
        let root = Self::apply_range(tree.root_node(), params.range.as_ref());

        let query = Query::new(&entry.language, &params.query).map_err(|e| {
            ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                format!(
                    "query compile error at byte offset {}: {} ({:?}). \
                     See the query_guide prompt for syntax help.",
                    e.offset, e.message, e.kind
                ),
                None,
            )
        })?;

        let mut cursor = QueryCursor::new();
        let mut matches_iter = cursor.matches(&query, root, source.as_bytes());
        let mut matches = Vec::new();
        while let Some(m) = matches_iter.next() {
            let captures = m
                .captures
                .iter()
                .map(|c| Capture {
                    name: query.capture_names()[c.index as usize].to_string(),
                    start_byte: c.node.start_byte(),
                    end_byte: c.node.end_byte(),
                    start_point: (c.node.start_position().row, c.node.start_position().column),
                    end_point: (c.node.end_position().row, c.node.end_position().column),
                    text: c
                        .node
                        .utf8_text(source.as_bytes())
                        .unwrap_or("<invalid utf8>")
                        .to_string(),
                })
                .collect();
            matches.push(QueryMatch {
                pattern_index: m.pattern_index,
                captures,
            });
        }

        let json = serde_json::to_string_pretty(&matches).map_err(|e| {
            ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                format!("failed to serialize query matches: {e}"),
                None,
            )
        })?;
        Ok(CallToolResult::success(vec![ContentBlock::text(json)]))
    }

    #[tool(
        description = "Find the smallest named node at a byte offset in a source file, \
                        returning it and its ancestor chain up to the root. \
                        Read-only; does not modify the file.",
        annotations(
            title = "Find Node At Offset",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn tree_sitter_find_node(
        &self,
        Parameters(params): Parameters<FindNodeParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let (source, tree) = self.load_tree(&params.path, params.language.as_deref())?;

        let root = tree.root_node();
        if params.byte > root.end_byte() {
            return Err(ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                format!(
                    "byte offset {} is past the end of the file ({} bytes)",
                    params.byte,
                    root.end_byte()
                ),
                None,
            ));
        }

        let mut node = root
            .descendant_for_byte_range(params.byte, params.byte)
            .unwrap_or(root);

        let mut ancestors = Vec::new();
        loop {
            ancestors.push(Self::node_info(node, &source));
            match node.parent() {
                Some(parent) => node = parent,
                None => break,
            }
        }

        let json = serde_json::to_string_pretty(&FindNodeResult { ancestors }).map_err(|e| {
            ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                format!("failed to serialize node info: {e}"),
                None,
            )
        })?;
        Ok(CallToolResult::success(vec![ContentBlock::text(json)]))
    }

    #[tool(
        description = "List the languages with available tree-sitter grammars on this server, \
                        along with the file extensions used to infer each one.",
        annotations(
            title = "List Supported Languages",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn tree_sitter_list_languages(&self) -> Result<CallToolResult, ErrorData> {
        let mut ids = self.sorted_language_ids();
        ids.sort_unstable();
        let lines: Vec<String> = ids
            .iter()
            .map(|id| {
                let ext = self.languages[id].extensions.join(", ");
                format!("{id}: .{}", ext.replace(", ", ", ."))
            })
            .collect();
        Ok(CallToolResult::success(vec![ContentBlock::text(
            lines.join("\n"),
        )]))
    }
}

#[prompt_router]
impl TreeSitterServer {
    #[prompt(
        name = "query_guide",
        description = "Guide for writing tree-sitter S-expression queries across the languages this server supports"
    )]
    async fn query_guide(&self) -> Result<GetPromptResult, ErrorData> {
        let langs = self.sorted_language_ids();
        let text = format!(
            "Tree-sitter S-expression query syntax:\n\
             \n\
             - `(node_kind)` matches any node of that kind.\n\
             - `(node_kind field: (child_kind) @capture_name)` matches a field and captures it.\n\
             - `@capture_name` right after a node pattern captures the whole node.\n\
             - `_` matches any node kind (wildcard).\n\
             - `[(a) (b)] @capture` matches either alternative.\n\
             - `(a) . (b)` anchors `b` to immediately follow `a` among siblings.\n\
             - Predicates filter matches by captured text: \
               `(#eq? @capture \"literal\")`, `(#match? @capture \"regex\")`.\n\
             \n\
             Supported languages on this server: {langs:?}. \
             Use tree_sitter_dump_ast first to see the node kinds and field names \
             for the file you're targeting, then write a query against tree_sitter_run_query.",
        );
        Ok(GetPromptResult::new(vec![PromptMessage::new_text(
            Role::User,
            text,
        )]))
    }
}

#[tool_handler]
#[prompt_handler]
impl ServerHandler for TreeSitterServer {
    fn get_info(&self) -> rmcp::model::ServerInfo {
        rmcp::model::ServerInfo::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_prompts()
                .enable_resources()
                .build(),
        )
        .with_server_info(Implementation::new("tree-sitter-mcp", "0.1.0"))
        .with_instructions(
            "Parse and analyze source code files using tree-sitter grammars. \
             Start with tree_sitter_list_languages to see what's supported, \
             tree_sitter_dump_ast to explore a file's structure, then \
             tree_sitter_run_query or tree_sitter_find_node for targeted lookups.",
        )
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, ErrorData> {
        Ok(ListResourcesResult::with_all_items(vec![
            Resource::new("tree-sitter://languages", "Supported Languages")
                .with_description("List of languages with available grammars")
                .with_mime_type("text/plain"),
        ]))
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResult, ErrorData> {
        match request.uri.as_str() {
            "tree-sitter://languages" => {
                let ids = self.sorted_language_ids();
                let text = ids.join("\n");
                Ok(ReadResourceResult::new(vec![
                    ResourceContents::TextResourceContents {
                        uri: "tree-sitter://languages".into(),
                        text,
                        mime_type: Some("text/plain".into()),
                        meta: None,
                    },
                ]))
            }
            _ => Err(ErrorData::resource_not_found(
                format!(
                    "Resource not found: {}. Available: tree-sitter://languages",
                    request.uri
                ),
                None,
            )),
        }
    }

    async fn list_resource_templates(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourceTemplatesResult, ErrorData> {
        Ok(ListResourceTemplatesResult::with_all_items(vec![
            ResourceTemplate::new("file://{path}", "File Source")
                .with_description("Source code content of any file accessible to the server")
                .with_mime_type("text/plain"),
        ]))
    }
}
