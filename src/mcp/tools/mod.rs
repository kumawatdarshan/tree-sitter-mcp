pub mod dump_ast;
pub mod find_node;
pub mod list_languages;
pub mod ping;
pub mod run_query;

use std::sync::Arc;

use rmcp::{
    ErrorData, RoleServer, ServerHandler,
    handler::server::router::tool::ToolRouter,
    model::{
        ErrorCode, Implementation, ListResourceTemplatesResult, ListResourcesResult,
        PaginatedRequestParams, ReadResourceRequestParams, ReadResourceResult, Resource,
        ResourceContents, ResourceTemplate, ServerCapabilities,
    },
    prompt_handler,
    service::RequestContext,
    tool_handler,
};

use crate::grammar::{GrammarEngine, GrammarError};

#[derive(Clone)]
pub struct TreeSitterServer {
    pub(crate) grammar: Arc<GrammarEngine>,
    tool_router: ToolRouter<Self>,
}

impl TreeSitterServer {
    pub fn new(grammar: Arc<GrammarEngine>) -> Self {
        Self {
            grammar,
            tool_router: Self::tool_router(),
        }
    }

    fn tool_router() -> ToolRouter<Self> {
        Self::ping_router()
            + Self::dump_ast_router()
            + Self::run_query_router()
            + Self::find_node_router()
            + Self::list_languages_router()
    }
}

pub(crate) fn grammar_error(error: GrammarError) -> ErrorData {
    let code = match &error {
        GrammarError::UnknownLanguage { .. }
        | GrammarError::LanguageInference { .. }
        | GrammarError::SourceRead { .. }
        | GrammarError::Query { .. }
        | GrammarError::ByteOutOfBounds { .. } => ErrorCode::INVALID_PARAMS,

        _ => ErrorCode::INTERNAL_ERROR,
    };

    ErrorData::new(code, error.to_string(), None)
}

#[tool_handler(router = self.tool_router)]
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
        .with_server_info(Implementation::new("tree-sitter-mcp", "0.0.1"))
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
        let uri = request.uri.as_str();
        match uri {
            "tree-sitter://languages" => {
                let summaries = self.grammar.language_summaries();
                let text = summaries
                    .iter()
                    .filter(|s| s.loaded)
                    .map(|s| s.id.as_str())
                    .collect::<Vec<_>>()
                    .join("\n");
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
                format!("Resource not found: {uri}. Available: tree-sitter://languages"),
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
