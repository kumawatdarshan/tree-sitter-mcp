use grammar::{GrammarEngine, GrammarError};
use rmcp::{
    ErrorData,
    handler::{server::router::tool::ToolRouter, server::tool::IntoCallToolResult},
    model::CallToolResult,
};
use std::sync::Arc;

pub(crate) mod prompts;
pub(crate) mod tools;

#[derive(Clone, Debug)]
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
            + Self::capabilities_router()
            + Self::run_query_router()
            + Self::dump_ast_router()
            + Self::find_node_router()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum McpError {
    #[error(transparent)]
    Grammar(#[from] GrammarError),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Rmcp(#[from] ErrorData),
}

impl IntoCallToolResult for McpError {
    fn into_call_tool_result(self) -> Result<CallToolResult, ErrorData> {
        Err(match self {
            McpError::Grammar(e) => ErrorData::internal_error(format!("{e}"), None),
            McpError::Io(e) => ErrorData::internal_error(format!("{e}"), None),
            McpError::Rmcp(e) => e,
        })
    }
}
