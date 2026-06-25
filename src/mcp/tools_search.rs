#[macro_export]
macro_rules! tools_search {
    () => {
    #[tool(
        name = "unified_search",
        description = "Search across windows, apps, files, clipboard history, and audit log in one query. Returns scored results ranked by relevance.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn unified_search(
        &self,
        Parameters(args): Parameters<SearchArgs>,
    ) -> String {
        self.exec("search.query", serde_json::json!({
                "query": args.query,
                "categories": args.categories,
                "limit": args.limit,
            }),).await
    }

    #[tool(
        name = "search_index_status",
        description = "Get search index statistics — indexed file count and last index time.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn search_index_status(&self) -> String {
        self.exec("search.index", serde_json::json!({}),).await
    }
    };
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
pub(crate) struct SearchArgs {
    pub query: String,
    #[serde(default)]
    pub categories: Option<Vec<String>>,
    #[serde(default)]
    pub limit: Option<usize>,
}
