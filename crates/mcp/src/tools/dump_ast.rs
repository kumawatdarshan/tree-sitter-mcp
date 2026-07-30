use grammar::ParseSession;

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
        let source = std::fs::read_to_string(&params.file.path)?;
        let lang = self
            .grammar
            .resolve_language(&params.file.path, params.file.language.as_deref())?;
        let ast = ParseSession::new(lang.clone(), source)?.dump_ast(params.range);

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
