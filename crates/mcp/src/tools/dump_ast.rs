use crate::{
    McpError,
    tools::{FileParams, text_result},
};
use rmcp::{
    handler::server::wrapper::Parameters, model::CallToolResult, schemars, tool, tool_router,
};
use serde::Deserialize;

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
    ) -> Result<CallToolResult, McpError> {
        let ast = self.grammar.dump_ast(
            &params.file.path,
            params.file.language.as_deref(),
            params.range,
        )?;

        Ok(text_result(ast))
    }
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(crate) struct DumpAstParams {
    #[serde(flatten)]
    pub file: FileParams,

    #[schemars(
        description = "Restrict the dump to the smallest node covering this byte range, instead of the whole file"
    )]
    pub range: Option<std::ops::Range<usize>>,
}
