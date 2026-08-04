use std::ops::Range;

use grammar::ParseSession;
use rmcp::{
    handler::server::wrapper::Parameters, model::CallToolResult, schemars, tool, tool_router,
};
use serde::Deserialize;

use crate::{
    McpError,
    tools::{FileParams, json_result},
};

#[tool_router(router = run_query_router, vis = "pub(crate)")]
impl crate::TreeSitterServer {
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
    ) -> Result<CallToolResult, McpError> {
        let source = std::fs::read_to_string(&params.file.path)?;
        let lang = self
            .grammar
            .resolve_language(&params.file.path, params.file.language.as_deref())?;
        let matches = ParseSession::new(lang, source)?.run_query(&params.query, params.range)?;

        json_result(&matches, "query matches")
    }
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(crate) struct RunQueryParams {
    #[serde(flatten)]
    pub file: FileParams,

    #[schemars(
        description = r#"A tree-sitter S-expression query, e.g. "(function_item name: (identifier) @name)". Use the query_guide prompt for syntax help."#
    )]
    pub query: String,

    #[schemars(description = "Restrict the search to this byte range instead of the whole file")]
    pub range: Option<Range<usize>>,
}
