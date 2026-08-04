pub(crate) mod capabilities;
pub(crate) mod dump_ast;
pub(crate) mod find_node;
pub(crate) mod ping;
pub(crate) mod run_query;

use crate::{McpError, TreeSitterServer};
use rmcp::{
    ErrorData, ServerHandler,
    model::{CallToolResult, ContentBlock, ErrorCode, Implementation, ServerCapabilities},
    prompt_handler, schemars, tool_handler,
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

/// Hold an optional explicit language id, so we can pass `Option<&LanguageId>`
/// into the grammar engine without double-parsing.
pub(crate) struct ResolvedLanguage(pub Option<grammar::LanguageId>);

impl ResolvedLanguage {
    pub fn from_params(language: &Option<String>) -> Result<Self, McpError> {
        Ok(Self(match language {
            Some(s) => Some(grammar::LanguageId::new(s.clone()).map_err(|e| {
                McpError::Rmcp(ErrorData::new(
                    ErrorCode::INVALID_PARAMS,
                    format!("invalid language id `{s}`: {e}"),
                    None,
                ))
            })?),
            None => None,
        }))
    }

    pub fn as_ref(&self) -> Option<&grammar::LanguageId> {
        self.0.as_ref()
    }
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
                .build(),
        )
        .with_server_info(Implementation::new("tree-sitter-mcp", "0.0.1"))
        .with_instructions(
            "Parse and analyze source code files using tree-sitter grammars. \
             Start with tree_sitter_get_capabilities to load the languages you need \
             and see what's supported, then tree_sitter_dump_ast to explore a file's \
             structure, then tree_sitter_run_query or tree_sitter_find_node for targeted lookups.",
        )
    }
}
