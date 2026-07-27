use rmcp::{
    ErrorData,
    handler::server::wrapper::Parameters,
    model::CallToolResult,
    schemars, tool, tool_router,
};
use serde::Deserialize;

use crate::grammar::ByteRange;
use crate::mcp::tools::{grammar_error, text_result};

#[tool_router(router = dump_ast_router, vis = "pub(crate)")]
impl crate::TreeSitterServer {
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
        let ast = self
            .grammar
            .dump_ast(
                &params.path,
                params.language.as_deref(),
                params.range.as_ref(),
            )
            .map_err(grammar_error)?;

        Ok(text_result(ast))
    }
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(crate) struct DumpAstParams {
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
