#[macro_export]
macro_rules! tools_secrets {
    () => {
        #[tool(
            name = "secrets_list_collections",
            description = "List all keyring collections. Returns available secret collections from the Secret Service.",
            annotations(
                read_only_hint = true,
                destructive_hint = false,
                idempotent_hint = true,
                open_world_hint = true
            )
        )]
        async fn secrets_list_collections(&self) -> String {
            self.exec("secrets.list_collections", json!({})).await
        }

        #[tool(
            name = "secrets_get_secret",
            description = "Look up a secret by its attributes (key=value pairs). Requires confirmation approval before returning the secret value.",
            annotations(
                read_only_hint = false,
                destructive_hint = false,
                idempotent_hint = true,
                open_world_hint = true
            )
        )]
        async fn secrets_get_secret(
            &self,
            Parameters(SecretsGetArgs { attributes }): Parameters<SecretsGetArgs>,
        ) -> String {
            self.exec("secrets.get_secret", json!({"attributes": attributes})).await
        }

        #[tool(
            name = "secrets_store_secret",
            description = "Store a secret in the keyring. Requires confirmation approval.",
            annotations(
                read_only_hint = false,
                destructive_hint = true,
                idempotent_hint = false,
                open_world_hint = true
            )
        )]
        async fn secrets_store_secret(
            &self,
            Parameters(SecretsStoreArgs {
                attributes,
                secret,
                label,
                collection,
            }): Parameters<SecretsStoreArgs>,
        ) -> String {
            self.exec("secrets.store_secret", json!({
                "attributes": attributes,
                "secret": secret,
                "label": label,
                "collection": collection,
            })).await
        }
    };
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
pub(crate) struct SecretsGetArgs {
    /// Key=value pairs that identify the secret
    pub attributes: std::collections::HashMap<String, String>,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
pub(crate) struct SecretsStoreArgs {
    /// Key=value pairs that identify the secret
    pub attributes: std::collections::HashMap<String, String>,
    /// The secret value to store
    pub secret: String,
    /// Optional human-readable label
    #[serde(default)]
    pub label: Option<String>,
    /// Optional collection path (e.g. "/org/freedesktop/secrets/collection/login")
    #[serde(default)]
    pub collection: Option<String>,
}
