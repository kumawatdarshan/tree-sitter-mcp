pub(crate) mod dump_ast;
pub(crate) mod find_node;
pub(crate) mod list_languages;
pub(crate) mod ping;
pub(crate) mod run_query;

use crate::{McpError, TreeSitterServer};
use rmcp::{
    ErrorData, RoleServer, ServerHandler,
    model::{
        CallToolResult, ContentBlock, ErrorCode, Implementation, ListResourcesResult,
        PaginatedRequestParams, ReadResourceRequestParams, ReadResourceResult, Resource,
        ResourceContents, ServerCapabilities,
    },
    prompt_handler, schemars,
    service::RequestContext,
    tool_handler,
};
use serde::Deserialize;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(crate) struct FileParams {
    #[schemars(description = "workspace-relative path to the source file")]
    pub path: String,

    #[schemars(description = "Language id. Inferred from the file extension if omitted.")]
    pub language: Option<String>,
}

pub(crate) fn text_result(text: impl Into<String>) -> CallToolResult {
    CallToolResult::success(vec![ContentBlock::text(text)])
}

pub(crate) fn json_result<T: serde::Serialize>(
    value: &T,
    context: &str,
) -> Result<CallToolResult, McpError> {
    let json = serde_json::to_string_pretty(value).map_err(|e| {
        ErrorData::new(
            ErrorCode::INTERNAL_ERROR,
            format!("failed to serialize {context}: {e}"),
            None,
        )
    })?;
    Ok(CallToolResult::success(vec![ContentBlock::text(json)]))
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
                let text = self.grammar.available_ids().join("\n");

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
}
