# Secret/Keyring Access Design

**Deskbrid Issue #29**

## Overview
Provide agents secure access to stored credentials (GNOME Keyring, KDE KWallet) through the Deskbrid protocol. Agents need this for authenticated API calls, GitHub tokens, etc. — without storing secrets in plaintext configs.

## Background

### GNOME Keyring and the Secret Service API
GNOME Keyring implements the [freedesktop.org Secret Service API](https://specifications.freedesktop.org/secret-service/latest), a D-Bus based interface for storing secrets. The service provides:
- `org.freedesktop.Secret.Service`: Manages sessions and collections.
- `org.freedesktop.Secret.Collection`: Contains items (secrets).
- `org.freedesktop.Secret.Item`: Holds a secret, lookup attributes, and a label.
- `org.freedesktop.Secret.Session`: Tracks state between service and client.
- `org.freedesktop.Secret.Prompt`: Used when user interaction is needed (e.g., unlocking).

The Secret Service API uses Diffie-Hellman key exchange to securely transfer secrets between the service and client after authentication. Collections are unlocked automatically when the user logs in (if configured).

### KDE KWallet
KDE KWallet now provides a thin compatibility layer that implements the Secret Service API, translating its native DBus calls to the Secret Service interface. This means:
- Both GNOME Keyring and KDE KWallet expose the same `org.freedesktop.Secret.*` DBus interface.
- Applications targeting the Secret Service API work with either backend seamlessly.
- The underlying storage differs (KWallet vs. GNOME Keyring), but the API is unified.

**Conclusion**: We can target the Secret Service API exclusively and support both keyrings without conditional logic.

## How GNOME Keyring Works via DBus
1. **Service Discovery**: The Secret Service is available at `org.freedesktop.secrets` on the session bus.
2. **Session Establishment**: 
   - Client calls `OpenSession` on the Service interface.
   - Service responds with session parameters and a DH public key.
   - Client performs DH key exchange to derive an encryption key.
3. **Collection Access**: 
   - Client calls `GetCollections` to list available collections (e.g., `login`, `session`).
   - Default collection (`login`) is typically unlocked on login.
4. **Item Operations**:
   - To store: Client calls `CreateItem` on a Collection with attributes, secret, and label.
   - To lookup: Client calls `SearchItems` with attributes to find matching items, then `GetSecret` to retrieve the encrypted secret (decrypted client-side using the session key).
5. **Prompts**: If the collection is locked, the service returns a `Prompt` requiring user action (via `Prompt` interface) to unlock.

## How KDE KWallet Differs
- **Protocol**: KWallet now implements the Secret Service API directly; no separate protocol needed.
- **Behavior**: 
  - Unlock timing may differ (KWallet integrates with KDE Plasma wallet).
  - Underlying storage is KWallet files instead of GNOME Keyring's.
  - UI prompts for unlocking are KWallet-native.
- **Unification**: Yes — the Secret Service API abstracts these differences. A client using the API works identically with both backends.

## Minimum Viable API Surface
For agent credential access, we need:
1. **List Collections**: Discover available collections (optional but useful).
2. **Get Secret by Attributes**: Retrieve a secret using lookup attributes (non-secret key-value pairs).
3. **Store Secret**: Save a secret with attributes and a human-readable label.

We intentionally omit:
- Listing all items in a collection (prevents credential fishing).
- Modifying/deleting existing secrets (reduce attack surface; agents can overwrite by storing new).
- Direct session management (handled internally by Deskbrid).

Attributes serve as the lookup key; they are **not secret** (stored plaintext) but should be unique enough to avoid collisions. Label is for display only.

## Security Scoping
### What Agents Should NOT Be Able To Do
- **Bulk Read**: Cannot enumerate all secrets in a collection (no `ListItems` equivalent).
- **Blind Write**: Cannot store without specifying attributes (prevents accidental overwrites).
- **Privilege Escalation**: Cannot access collections of other users or system keyrings (limited to session bus).
- **Secret Exfiltration via Protocol**: Secrets never leave the Deskbrid daemon in plaintext; they are used only for immediate agent requests (e.g., filling a GitHub token in a prompt) and not returned unless explicitly requested via `get_secret` (and even then, only after confirmation).

### Leak Vectors to Call Out
- **Memory Leaks**: Secrets held in daemon memory could be swapped or core-dumped. Mitigation: zero memory after use.
- **Unlocked Keyring**: If the user's keyring is unlocked (typical on login), any process as the user can access Secret Service directly. Deskbrid adds a consent layer but doesn't replace OS-level security.
- **Attribute Exposure**: Attributes are visible to any process querying the Secret Service. Do not store sensitive data in attributes.
- **Confirmation Spoofing**: A compromised agent could mimic confirmation requests. Mitigation: bind confirmations to agent session IDs.

## Interaction with Confirmation System
**Yes, credential reads and writes should require confirmation.** 
- **Rationale**: Reading a secret is as sensitive as writing it — both risk credential exposure. Tuck's confirmation system (for destructive actions) should extend to sensitive operations like secret access.
- **Implementation**:
  - `secrets.get_secret` and `secrets.store_secret` trigger a confirmation prompt.
  - Prompt details: 
    - For get: `"Agent [ID] requests to read secret labeled '[label]' (attributes: [attrs]). Allow?"`
    - For store: `"Agent [ID] requests to store secret labeled '[label]' (attributes: [attrs]). Allow?"`
  - User can approve, deny, or set temporary/permanent rules (e.g., "allow this agent to read GitHub tokens for 1 hour").
  - Integrates with existing confirmation UI (dashboard, popup, or CLI).
- **Note**: If the keyring is locked, the Secret Service will already prompt to unlock — Deskbrid's confirmation is an additional layer for consent.

## Recommended Protocol Action Names
We extend the Deskbrid protocol with a `secrets` namespace. All actions require a unique `id` for tracking.

### `secrets.list_collections`
```json
{
  "type": "secrets.list_collections",
  "id": "<uuid>"
}
```
**Response**:
```json
{
  "type": "secrets.list_collections",
  "id": "<uuid>",
  "collections": [
    { "path": "/org/freedesktop/secrets/collection/login", "label": "Login Keyring", "locked": false },
    { "path": "/org/freedesktop/secrets/collection/session", "label": "Session Keyring", "locked": true }
  ]
}
```

### `secrets.get_secret`
```json
{
  "type": "secrets.get_secret",
  "id": "<uuid>",
  "attributes": { "service": "github", "username": "tuck" }
}
```
**Response** (after confirmation):
```json
{
  "type": "secrets.get_secret",
  "id": "<uuid>",
  "secret": "ghp_...",  // plaintext secret
  "label": "GitHub token for Tuck"
}
```
*Error*: `denied` if confirmation rejected, `not_found` if no matching item.

### `secrets.store_secret`
```json
{
  "type": "secrets.store_secret",
  "id": "<uuid>",
  "collection": "/org/freedesktop/secrets/collection/login",  // optional; defaults to login
  "attributes": { "service": "github", "username": "tuck" },
  "secret": "ghp_...",
  "label": "GitHub token for Tuck"
}
```
**Response** (after confirmation):
```json
{
  "type": "secrets.store_secret",
  "id": "<uuid>",
  "success": true
}
```
*Error*: `denied` if confirmation rejected.

## MCP Tool Signatures
For MCP integration, expose these as Deskbrid tools:

```typescript
// List available collections
async function listCollections(): Promise<{ path: string; label: string; locked: boolean }[]>

// Retrieve a secret by attributes (requires user confirmation)
async function getSecret(attributes: Record<string, string>): Promise<{ secret: string; label: string }>

// Store a secret with attributes and label (requires user confirmation)
async function storeSecret(
  attributes: Record<string, string>,
  secret: string,
  label?: string,
  collection?: string
): Promise<void>
```

## Security Notes
- **zbus Usage**: Leverages existing zbus dependency for safe DBus interaction (no raw DBus calls).
- **No Plaintext Logging**: Secrets are never logged; attributes may be logged for debugging (non-sensitive).
- **Memory Safety**: Secrets zeroed from memory immediately after use in protocol response.
- **Scope Limitation**: Actions restricted to user's session bus; cannot access system or other users' keyrings.
- **Fallback**: If Secret Service unavailable, return `service_unavailable` error.
- **Rate Limiting**: The `secrets.get_secret` action is subject to Deskbrid's existing rate limiter to prevent credential fishing via rapid repeated calls.
- **Audit Logging**: All `secrets.get_secret` and `secrets.store_secret` calls are logged to the audit log with agent session ID and attributes (secrets themselves are never logged).

## Open Questions
- Should we allow agents to create/delete collections? (No — too powerful; use existing collections.)
- Should we support item modification/deletion? (No — store secret with same attributes to overwrite.)
- How to handle expired/temporary secrets? (Out of scope; rely on agent to manage rotation.)

---
*This spec assumes the Secret Service API is available on the user's session bus. Deskbrid daemon will handle session setup and attribute-based lookup internally.*