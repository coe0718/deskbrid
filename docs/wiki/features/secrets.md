# Secrets / Keyring Access

Store, retrieve, and list secrets using the system keyring (Secret Service API / D-Bus).

## Actions

### secrets.list_collections

List available secret collections in the keyring.

```bash
deskbrid secrets.list_collections
```

No parameters.

Response:

```json
{
  "type": "response",
  "status": "ok",
  "data": [
    {"name": "login", "label": "Login", "locked": false},
    {"name": "default", "label": "Default", "locked": false}
  ]
}
```

### secrets.get_secret

Retrieve a secret by its attributes (key-value pairs).

| Parameter     | Type              | Description                   |
|---------------|-------------------|-------------------------------|
| `attributes`  | map[string,string] | Keyring attributes to match  |

```bash
deskbrid secrets.get_secret '{"attributes": {"service": "deskbrid", "account": "user"}}'
```

```json
{
  "type": "secrets.get_secret",
  "attributes": {
    "service": "deskbrid",
    "account": "user"
  }
}
```

Response:

```json
{
  "type": "response",
  "status": "ok",
  "data": {
    "attributes": {"service": "deskbrid", "account": "user"},
    "secret": "my-api-key-12345",
    "label": "Deskbrid API Key",
    "collection": "login"
  }
}
```

### secrets.store_secret

Store a new secret in the keyring.

| Parameter    | Type              | Description                    |
|--------------|-------------------|--------------------------------|
| `attributes` | map[string,string]| Keyring attributes            |
| `secret`     | string            | The secret value to store     |
| `label`      | string?           | Human-readable label (optional)|
| `collection` | string?           | Collection name (optional, default `login`) |

```bash
deskbrid secrets.store_secret '{"attributes": {"service": "github", "account": "user"}, "secret": "ghp_abc123", "label": "GitHub Token"}'
```

```json
{
  "type": "secrets.store_secret",
  "attributes": {"service": "github", "account": "user"},
  "secret": "ghp_abc123",
  "label": "GitHub Token",
  "collection": "login"
}
```

## Python Example

```python
from deskbrid import Deskbrid

client = Deskbrid()

# Store a secret
client.secrets_store_secret(
    attributes={"service": "my-app", "username": "bot"},
    secret="supersecret123",
    label="My App Bot Token"
)

# Retrieve
result = client.secrets_get_secret(
    attributes={"service": "my-app", "username": "bot"}
)
print(f"Found: {result['label']} = {result['secret'][:4]}...")
```

## Requirements

- D-Bus Secret Service (`org.freedesktop.secrets`) — supported by GNOME
  Keyring, KDE Wallet, KeePassXC, and `secret-tool`.
- The `secret-tool` CLI utility (libsecret) may be used as fallback.

## Safety

- Secrets are stored encrypted in the system keyring, not in Deskbrid's config.
- Access to the keyring requires the user's keyring to be unlocked.
- Confirmation mode may require approval for secret read/write operations.

## Current Status

**Experimental** — v1.0.0 feature.
