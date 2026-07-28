use rmcp::{
    ErrorData,
    model::{GetPromptResult, PromptMessage, Role},
    prompt, prompt_router,
};

#[prompt_router(vis = "pub")]
impl crate::TreeSitterServer {
    #[prompt(
        name = "query_guide",
        description = "Guide for writing tree-sitter S-expression queries across the languages this server supports"
    )]
    async fn query_guide(&self) -> Result<GetPromptResult, ErrorData> {
        let ids = self
            .grammar
            .loaded_language_ids()
            .collect::<Vec<_>>()
            .join(", ");
        let text = format!(
            r#"Tree-sitter S-expression query syntax:

             - `(node_kind)` matches any node of that kind.
             - `(node_kind field: (child_kind) @capture_name)` matches a field and captures it.
             - `@capture_name` right after a node pattern captures the whole node.
             - `_` matches any node kind (wildcard).
             - `[(a) (b)] @capture` matches either alternative.
             - `(a) . (b)` anchors `b` to immediately follow `a` among siblings.
             - Predicates filter matches by captured text:
               `(#eq? @capture "literal")`, `(#match? @capture "regex")`.

             Supported languages on this server: {ids}.
             Use tree_sitter_dump_ast first to see the node kinds and field names
             for the file you're targeting, then write a query against tree_sitter_run_query."#,
        );
        Ok(GetPromptResult::new(vec![PromptMessage::new_text(
            Role::User,
            text,
        )]))
    }
}
