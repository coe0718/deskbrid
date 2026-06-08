# Secrets

Deskbrid v1.0.0 provides a secret/keyring service backed by GNOME Keyring or the
compatible Secret Service D-Bus implementation. All secret access flows through
the confirmation UI by default to prevent credential fishing.

## Actions

- `secrets.list_collections`
- `secrets.search_items`
- `secrets.get_secret`
- `secrets.store_secret`
- `secrets.delete_secret`

## List collections

```bash
deskbrid secrets.list_collections {}
```

## Search items

```bash
deskbrid secrets.search_items {
  query: "api-key",
  collection: "login"
}
```

## Read secret

```bash
deskbrid secrets.get_secret {
  item_id: "abcd-1234",
  collection: "login"
}
```

## Store secret

```bash
deskbrid secrets.store_secret {
  collection: "login",
  attributes: { service: "openai", account: "me" },
  secret: "sk-..."
}
```

## Auth

Secret actions require an active `confirm.challenge` / `confirm.resolve` flow
unless the current session is explicitly authorized by policy. This matches the
daemon's default `system.*` gating behavior.
