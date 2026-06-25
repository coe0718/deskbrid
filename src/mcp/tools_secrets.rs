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
            self.call_state( |state| {
                Box::pin(async move {
                    let action = $crate::protocol::Action::SecretsListCollections;
                    $crate::daemon::execute_secrets::execute_secrets_action(action, &state).await
                })
            }).await
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
            let attrs = attributes.clone();
            self.call_state( move |state| {
                Box::pin(async move {
                    let action = $crate::protocol::Action::SecretsGetSecret {
                        attributes: attrs,
                    };
                    $crate::daemon::execute_secrets::execute_secrets_action(action, &state).await
                })
            }).await
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
            let attrs = attributes.clone();
            let sec = secret.clone();
            let lbl = label.clone();
            let col = collection.clone();
            self.call_state( move |state| {
                Box::pin(async move {
                    let action = $crate::protocol::Action::SecretsStoreSecret {
                        attributes: attrs,
                        secret: sec,
                        label: lbl,
                        collection: col,
                    };
                    $crate::daemon::execute_secrets::execute_secrets_action(action, &state).await
                })
            }).await
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
