# Deskbrid: Proxmox VE Integration

**Goal:** Give Tuck full control over Jeremy's Proxmox host — VMs, LXCs, storage, networking, cluster status — through the same deskbrid socket he uses to control the desktop. Tuck can query cluster state, start/stop containers, create new LXCs, check storage usage, and manage backups, all from the same connection.

---

## Part 1: What the Proxmox VE API Exposes

### Authentication

Two methods, both supported:

| Method | How | Best For |
|--------|-----|----------|
| **API Token** | `Authorization: PVEAPIToken=root@pam!name=secret` header | Stateless, least-privilege, preferred for deskbrid |
| **Ticket** | POST `/api2/json/access/ticket` → cookie `PVEAuthCookie` + `CSRFPreventionToken` header | Full root access, requires password |

**Recommendation:** Use API tokens. Generate one in the Proxmox UI (Datacenter → Permissions → API Tokens) with only the privileges deskbrid needs. Store the `id:secret` in the deskbrid config file.

### Base URL

```
https://<host>:8006/api2/json/
```

Self-signed certificates are the default. Deskbrid must support `--insecure` / `accept_invalid_certs`.

### API Resource Tree (What Matters)

```
/                           GET    Cluster index
├── access/
│   ├── ticket              POST   Create auth ticket (username+password)
│   └── users               GET    List users
├── cluster/
│   ├── resources           GET    ★ ALL resources — nodes, VMs, LXCs, storage in one call
│   ├── status              GET    Quorum status
│   ├── tasks               GET    Task history
│   ├── backup              GET/POST  Backup schedules
│   ├── config/join         POST   Join cluster
│   ├── ha/                 Various  HA management
│   ├── firewall/           Various  Datacenter firewall rules
│   ├── sdn/                Various  Software-defined networking
│   └── nextid              GET    Get next free VMID
├── nodes/
│   └── {node}/
│       ├── status          GET    ★ CPU, mem, uptime, ksm, load, io, cpuinfo
│       ├── qemu/           GET    ★ List QEMU VMs on this node
│       │   └── {vmid}/
│       │       ├── status/
│       │       │   ├── current    GET    ★ Running state (qmpstatus, cpus, mem, disk, net)
│       │       │   ├── start      POST   Start VM
│       │       │   ├── stop       POST   Stop VM
│       │       │   ├── reboot     POST   Reboot VM
│       │       │   ├── shutdown   POST   ACPI shutdown
│       │       │   ├── reset      POST   Hard reset
│       │       │   └── suspend    POST   Suspend to disk/ram
│       │       ├── config         GET/PUT/POST VM config (cores, memory, disks, net)
│       │       ├── snapshot       GET/POST Snapshot management
│       │       ├── migrate        POST   Live migration
│       │       ├── clone          POST   Clone VM
│       │       ├── resize         PUT    Resize disk
│       │       ├── template       POST   Convert to template
│       │       ├── vncproxy       POST   VNC console access
│       │       ├── rrd            GET    RRD metrics
│       │       └── firewall/      Various VM-level firewall
│       ├── lxc/            GET    ★ List LXC containers
│       │   └── {vmid}/
│       │       ├── status/
│       │       │   ├── current    GET    ★ Running state
│       │       │   ├── start      POST   Start container
│       │       │   ├── stop       POST   Stop container
│       │       │   ├── reboot     POST   Reboot container
│       │       │   ├── shutdown   POST   Clean shutdown
│       │       │   └── suspend    POST   Suspend
│       │       ├── config         GET/PUT Container config
│       │       ├── snapshot       GET/POST LXC snapshots
│       │       ├── template       POST   Convert to template
│       │       ├── resize         PUT    Resize rootfs/mountpoints
│       │       ├── clone          POST   Clone container
│       │       ├── migrate        POST   Migrate
│       │       ├── interfaces     GET    Network interfaces
│       │       ├── firewall/      Various LXC firewall
│       │       ├── rrd            GET    RRD metrics
│       │       └── move_volume    POST   Move volume
│       ├── storage/        GET    ★ Storage info seen by this node
│       ├── network/        GET/PUT Host network config
│       ├── dns             GET/PUT Node DNS config
│       ├── disks/          GET    ★ Physical disk listing (smartctl)
│       ├── apt/            GET/POST Package updates
│       ├── tasks/          GET    Node-specific tasks
│       ├── services/       GET    Service status (pveproxy, etc.)
│       ├── syslog          GET    Syslog
│       ├── journal         GET    Journal entries
│       ├── rrddata         GET    ★ RRD time-series data
│       ├── config          GET    Node config
│       ├── subscription    GET/POST Subscription status
│       ├── termproxy       POST   Shell access (xterm.js)
│       └── wakeonlan       POST   WoL from node
├── storage/
│   └── {storage}/          GET/PUT/DELETE  Global storage management
├── pools/
│   └── {pool}/             GET/POST/PUT/DELETE  Resource pools
└── version                 GET    Proxmox version
```

**Key insight:** `/cluster/resources` is the single most valuable endpoint. One call returns every node, VM, LXC, and storage pool with status, disk usage, CPU, and memory — making most "what's my cluster doing?" queries a single HTTP call.

---

## Part 2: Deskbrid Actions That Make Sense

All actions use the `proxmox.*` prefix, matching deskbrid's protocol convention.

### P0 — Read-Only Status (Phase 1)

These are safe, stateless, and immediately useful. Tuck asks "what's running?" and gets an answer.

```
proxmox.status              → GET /cluster/resources           — Full cluster inventory
proxmox.nodes               → GET /nodes                       — Node list
proxmox.node_status {node}  → GET /nodes/{node}/status         — CPU, mem, uptime for one node
proxmox.vms {node?}         → GET /nodes/{node}/qemu           — QEMU VMs (or all via cluster/resources)
proxmox.containers {node?}  → GET /nodes/{node}/lxc            — LXC containers
proxmox.guest_config {node, vmid, type}  → GET /nodes/{node}/{type}/{vmid}/config
proxmox.guest_status {node, vmid, type}  → GET .../status/current
proxmox.storage.list        → GET /storage                     — All storage pools
proxmox.storage.get {storage}  → GET /storage/{storage}        — Single pool
proxmox.disks {node}        → GET /nodes/{node}/disks/list     — Physical disks (smartctl)
proxmox.version             → GET /version
proxmox.tasks {node?}       → GET /cluster/tasks               — Recent task history
proxmox.apt.list {node}     → GET /nodes/{node}/apt/update     — Package updates available
proxmox.network {node}      → GET /nodes/{node}/network        — Host network interfaces
proxmox.dns {node}          → GET /nodes/{node}/dns            — DNS config
proxmox.subscription {node} → GET /nodes/{node}/subscription   — License status
```

### P1 — Lifecycle Actions (Phase 2)

Mutating operations. Every one returns a task UPID for async tracking.

```
proxmox.guest.start   {node, vmid, type}
proxmox.guest.stop    {node, vmid, type, force?}
proxmox.guest.reboot  {node, vmid, type}
proxmox.guest.shutdown {node, vmid, type}
proxmox.guest.reset   {node, vmid, type}
proxmox.guest.suspend {node, vmid, type}
proxmox.guest.resume  {node, vmid, type}
proxmox.guest.migrate {node, vmid, type, target}
proxmox.guest.exec   {node, vmid, type, command, stdin?}  — ★ Run command inside guest
```

### P2 — Administration (Phase 3)

```
proxmox.lxc.create    {node, vmid, ...params}   — Create LXC container
proxmox.vm.create     {node, vmid, ...params}   — Create QEMU VM
proxmox.guest.clone   {node, vmid, type, newid} — Clone a guest
proxmox.guest.delete  {node, vmid, type}        — Destroy a guest
proxmox.guest.resize  {node, vmid, type, disk, size} — Resize disk
proxmox.guest.config_set {node, vmid, type, ...} — Update config (cores, mem, etc.)
proxmox.snapshot.list   {node, vmid, type}
proxmox.snapshot.create {node, vmid, type, name}
proxmox.snapshot.rollback {node, vmid, type, name}
proxmox.snapshot.delete {node, vmid, type, name}
proxmox.backup.list   → GET /cluster/backup
proxmox.backup.create {node, vmid, type, storage, mode}
proxmox.storage.create {storage, type, ...params}
proxmox.storage.delete {storage}
```

### P3 — Advanced (Phase 4)

```
proxmox.ha.status           → HA resource status
proxmox.ha.migrate          → Migrate HA service
proxmox.firewall.get        → Read firewall rules
proxmox.firewall.set        → Update firewall rules
proxmox.sdn.list            → SDN zones/vnets
proxmox.cluster.config      → Cluster-wide config
proxmox.node.wakeonlan      → WoL a remote machine
proxmox.apk.update {node}   → Run apt updates on a node
```

### Design Patterns Observed

1. **type parameter** — `"qemu"` or `"lxc"` — avoids duplicating every guest action for VMs vs containers
2. **node is optional** where it can be inferred from cluster/resources
3. **Every mutating action returns a task UPID** — deskbrid should surface this so Tuck can poll for completion
4. **config payloads** follow Proxmox's parameter names directly (lowercase, snake_case)

---

## Part 3: Implementation Approach in Rust

### Crate Choice: Direct `reqwest` — Not a Proxmox SDK Crate

**Why not `leeca_proxmox` (0.3.0)?**
- Covers authentication, nodes, cluster/resources, and VMs
- LXC operations are on the roadmap for 0.4.0 (not yet implemented)
- Storage, network, tasks, snapshots, backups are all on the roadmap
- It's one dev's pre-1.0 project — version lock risk
- Deskbrid already has `reqwest` in `Cargo.toml`

**Why not `proxmox-api` (0.1.1)?**
- Generated bindings, 47K SLoC, way too heavy
- Last updated Apr 2024 — stale

**Why `reqwest` directly:**
- Deskbrid already depends on `reqwest = "0.12"` with `features = ["json"]`
- Proxmox API is REST/JSON — trivially wrapped with `reqwest::Client`
- Full control over what endpoints are exposed
- No dependency risk for an immature crate
- Can add `serde_json::Value` for request/response until structured types are stabilized

### Architecture

```
src/
├── proxmox/
│   ├── mod.rs            # ProxmoxClient struct, connection pool, auth
│   ├── auth.rs           # Ticket + API token auth, token refresh
│   ├── types.rs          # Common types: GuestType, GuestStatus, StorageInfo
│   └── endpoints.rs      # Thin wrappers for each API endpoint
├── protocol/
│   ├── parse/
│   │   └── proxmox.rs    # Parse proxmox.* → Action::ProxmoxStatus, etc.
│   └── mod.rs            # Add proxmox.* route to from_json()
├── daemon/
│   └── proxmox.rs        # Dispatch handler: Action → ProxmoxClient call → JSON
└── mcp/
    ├── tool_list.rs      # Add proxmox tools to MCP list
    └── tools.rs          # Bridge MCP calls to proxmox actions
```

### No Desktop Backend Required

Unlike windows/input/screenshot actions which need the `DesktopBackend` trait, Proxmox actions are pure HTTP calls. They don't touch the display server. The dispatch path is:

```
Unix socket → parse → Action::ProxmoxStatus {..}
→ dispatch_action_with_options()
→ execute_proxmox::execute_proxmox(action, state)
→ state.proxmox_client.get("/cluster/resources")?
→ serde_json::Value response
```

The `ProxmoxClient` lives on `DaemonState` alongside the desktop backend. It's constructed at daemon startup from config.

### ProxmoxClient

```rust
pub struct ProxmoxClient {
    http: reqwest::Client,
    base_url: String,         // "https://192.168.1.100:8006/api2/json"
    credentials: AuthMethod,
    csrf_token: Mutex<Option<(String, Instant)>>,
    ticket: Mutex<Option<(String, Instant)>>,
}

pub enum AuthMethod {
    ApiToken { id: String, secret: String },
    UsernamePassword { username: String, password: String, realm: String },
}

impl ProxmoxClient {
    pub fn new(host: &str, port: u16, auth: AuthMethod, accept_invalid_certs: bool) -> Result<Self>;
    pub async fn authenticate(&self) -> Result<()>;
    pub async fn request(&self, method: Method, path: &str, body: Option<Value>) -> Result<Value>;
    pub async fn get(&self, path: &str) -> Result<Value>;
    pub async fn post(&self, path: &str, body: Value) -> Result<Value>;
    pub async fn put(&self, path: &str, body: Value) -> Result<Value>;
    pub async fn delete(&self, path: &str) -> Result<Value>;
}
```

### Dependencies (New)

```toml
# Already present:
# reqwest = { version = "0.12", features = ["json"] }
# tokio = { version = "1", features = ["full"] }
# serde_json = "1"
# anyhow = "1"

# No new crates needed.
```

`reqwest` already handles TLS (via `rustls`), JSON serialization, connection pooling, timeouts — everything needed for a Proxmox REST client.

### Config

Proxmox connection details go in deskbrid's config TOML (or environment):

```toml
[proxmox]
host = "192.168.1.100"
port = 8006
accept_invalid_certs = true   # Self-signed PVE cert

# Option A: API token (preferred)
api_token_id = "root@pam!deskbrid"
api_token_secret = "xxxx-xxxx-xxxx"

# Option B: Username + password
# username = "root"
# password = "yourpass"
# realm = "pam"
```

### Config from env vars (for Docker/CI):

```bash
PROXMOX_HOST=192.168.1.100
PROXMOX_PORT=8006
PROXMOX_TOKEN_ID=root@pam!deskbrid
PROXMOX_TOKEN_SECRET=xxxx-xxxx
PROXMOX_INSECURE=true
```

---

## Part 4: Protocol Actions (Action enum additions)

Following deskbrid's existing pattern where each action is a variant:

```rust
// Read-only
ProxmoxStatus,
ProxmoxNodes,
ProxmoxNodeStatus { node: String },
ProxmoxGuests { node: Option<String>, guest_type: GuestType },
ProxmoxGuestConfig { node: String, vmid: u32, guest_type: GuestType },
ProxmoxGuestStatus { node: String, vmid: u32, guest_type: GuestType },
ProxmoxStorageList,
ProxmoxStorageGet { storage: String },
ProxmoxDisks { node: String },
ProxmoxVersion,
ProxmoxTasks { node: Option<String>, limit: Option<u32> },
ProxmoxAptList { node: String },
ProxmoxNetwork { node: String },
ProxmoxDns { node: String },
ProxmoxSubscription { node: String },

// Lifecycle
ProxmoxGuestAction {
    action: GuestAction,       // Start, Stop, Reboot, Shutdown, Reset, Suspend, Resume
    node: String,
    vmid: u32,
    guest_type: GuestType,
    force: Option<bool>,
},
ProxmoxGuestMigrate {
    node: String,
    vmid: u32,
    guest_type: GuestType,
    target: String,
    online: Option<bool>,
},
ProxmoxGuestExec {
    node: String,
    vmid: u32,
    guest_type: GuestType,
    command: String,           // Shell command to run inside the guest
    user: Option<String>,      // --user passthrough (defaults to root if omitted)
    stdin: Option<String>,     // StdIN to pipe to command
},

// Administration
ProxmoxGuestCreate {
    node: String,
    guest_type: GuestType,
    params: serde_json::Value,   // Proxmox's native parameter object
},
ProxmoxGuestClone {
    node: String,
    vmid: u32,
    guest_type: GuestType,
    newid: u32,
    target: Option<String>,
},
ProxmoxGuestDelete {
    node: String,
    vmid: u32,
    guest_type: GuestType,
    destroy_unreferenced_disks: Option<bool>,
},
ProxmoxGuestResize {
    node: String,
    vmid: u32,
    guest_type: GuestType,
    disk: String,
    size: String,              // "+10G" format
},
ProxmoxGuestConfigSet {
    node: String,
    vmid: u32,
    guest_type: GuestType,
    params: serde_json::Value,
},

// Snapshots
ProxmoxSnapshotList { node: String, vmid: u32, guest_type: GuestType },
ProxmoxSnapshotCreate {
    node: String, vmid: u32, guest_type: GuestType,
    snapname: String, description: Option<String>,
},
ProxmoxSnapshotRollback { node: String, vmid: u32, guest_type: GuestType, snapname: String },
ProxmoxSnapshotDelete { node: String, vmid: u32, guest_type: GuestType, snapname: String },

// Backups
ProxmoxBackupList,
ProxmoxBackupCreate {
    node: String, vmid: u32, guest_type: GuestType,
    storage: String, mode: Option<String>, compress: Option<String>,
},
```

### Shared Types

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum GuestType {
    #[serde(rename = "qemu")]
    Qemu,
    #[serde(rename = "lxc")]
    Lxc,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum GuestAction {
    #[serde(rename = "start")]
    Start,
    #[serde(rename = "stop")]
    Stop,
    #[serde(rename = "reboot")]
    Reboot,
    #[serde(rename = "shutdown")]
    Shutdown,
    #[serde(rename = "reset")]
    Reset,
    #[serde(rename = "suspend")]
    Suspend,
    #[serde(rename = "resume")]
    Resume,
}
```

---

## Part 5: MCP Tools

MCP tools expose proxmox actions to AI coding assistants. Same pattern as existing tools in `src/mcp/tool_list.rs` and `src/mcp/tools.rs`.

### P0 MCP Tools

```
proxmox_status           — Full cluster inventory (read-only)
proxmox_nodes            — List nodes with health
proxmox_containers       — List LXC containers
proxmox_vms              — List QEMU VMs
proxmox_guest_config     — Get VM/LXC config
proxmox_guest_status     — Get running state of a guest
proxmox_storage_list     — Storage pool overview
proxmox_version          — Proxmox version info
```

### P1 MCP Tools (destructive)

```
proxmox_guest_action     — Start/stop/reboot a guest
proxmox_guest_migrate    — Migrate guest to another node
proxmox_guest_exec       — Run command inside a container (via SSH+pct)
```

---

## Part 5a: Guest Exec — Running Commands Inside Containers

**The problem:** There is no REST API endpoint for running commands inside an LXC container. 
For QEMU VMs, Proxmox exposes `POST /nodes/{node}/qemu/{vmid}/agent/exec` (requires QEMU 
guest agent). For LXCs, the API only offers `termproxy` (xterm.js console) — no exec endpoint.

The Proxmox way to run commands inside a container is `pct exec <vmid> -- <command>`, which 
runs on the Proxmox host using LXC's native `attach` mechanism. Similarly for VMs without 
the guest agent, you'd use `qm guest exec <vmid> -- <command>`.

`POST /nodes/{node}/execute` is NOT a shell executor — it's a **batch API call runner** 
that chains multiple REST API calls into one async task. It cannot run arbitrary shell commands.

### Solution: SSH Into the Proxmox Host

Deskbrid uses SSH to run `pct exec` on the Proxmox host. The flow:

```
Tuck → deskbrid socket → parse → Action::ProxmoxGuestExec {node, vmid, command}
→ deskbrid SSHs into Proxmox host → runs "pct exec 100 -- ls -la /etc"
→ captures stdout/stderr/exit code → returns to Tuck
```

### Implementation

**New module:** `src/proxmox/exec.rs` — SSH-backed guest command execution.

```rust
pub async fn guest_exec(
    client: &ProxmoxClient,
    node: &str,
    vmid: u32,
    guest_type: GuestType,
    command: &str,
    stdin: Option<&str>,
) -> anyhow::Result<GuestExecResult> {
    let tool = match guest_type {
        GuestType::Lxc => "pct",
        GuestType::Qemu => "qm",
    };

    // SSH into the Proxmox host and run: pct exec <vmid> -- <command>
    let ssh_cmd = format!(
        "{} exec {} -- {}",
        tool, vmid,
        shell_escape::unix::escape(std::borrow::Cow::Borrowed(command))
    );

    let result = run_ssh_command(&client.ssh_config, &ssh_cmd, stdin).await?;

    Ok(GuestExecResult {
        exit_code: result.exit_code,
        stdout: result.stdout,
        stderr: result.stderr,
        truncated: result.truncated,
    })
}
```

**SSH config** goes in the deskbrid config alongside the Proxmox API credentials:

```toml
[proxmox]
host = "192.168.1.100"
port = 8006
accept_invalid_certs = true
api_token_id = "root@pam!deskbrid"
api_token_secret = "xxxx-xxxx-xxxx"

# SSH access to the Proxmox host (for pct exec / qm exec)
ssh_user = "root"
ssh_key_path = "~/.ssh/id_ed25519"
ssh_port = 22
```

**SSH crate:** Use `ssh2` (libssh2 bindings) or shell out to `ssh` binary. The shell-out approach 
is simpler and zero-dependency:

```rust
let output = tokio::process::Command::new("ssh")
    .args(["-i", &ssh_config.key_path, "-p", &ssh_config.port.to_string(),
           format!("{}@{}", ssh_config.user, client.host).as_str(),
           &ssh_cmd])
    .output()
    .await?;
```

**Dependency:** No new crates if shelling out to `ssh`. If using `ssh2` for native SSH:
```toml
ssh2 = { version = "0.9", optional = true }
```

### QEMU Guest Agent Alternative

For VMs that have the QEMU guest agent installed, the REST API endpoint is available as a 
cleaner alternative:

```
POST /nodes/{node}/qemu/{vmid}/agent/exec
Body: {"command": "my-command", "input-data": "optional stdin"}
```

Deskbrid should prefer the agent endpoint when `guest_type == Qemu` and the guest agent is 
responsive, falling back to `qm exec` via SSH when the agent is unavailable.

### Priority

`proxmox.guest_exec` belongs in **Phase 2 (Lifecycle)** — it's a bread-and-butter operation 
for Tuck managing containers. The SSH config is a Phase 1 foundation concern, the exec 
implementation is Phase 2.

### Security

- **Dedicated SSH key** — The SSH key for the Proxmox host MUST be a separate key stored
  outside the user's main `~/.ssh/` identity. If deskbrid's config is ever compromised,
  it must not also leak the user's primary SSH identity. Recommended path:

  ```toml
  [proxmox.exec]
  ssh_key = "~/.config/deskbrid/proxmox_exec_id_ed25519"
  ssh_user = "root"
  ssh_port = 22
  ```

- **No passwords in config** — SSH key-based auth only
- **Root by default** — `pct exec` runs as root inside the container. Deskbrid should
  passthrough the `--user` flag so Tuck can specify a non-root user:

  ```json
  {"type": "proxmox.guest.exec", "node": "pve", "vmid": 100, "guest_type": "lxc",
   "command": "whoami", "user": "www-data"}
  ```

  This maps to: `pct exec 100 --user www-data -- whoami`

- **Audit every exec** — full command text and target (node, vmid, user) in audit log
- **Consider `allowed_commands` allowlist** in config for production deployments
- **Key generation** — Deskbrid should provide a helper to generate the dedicated key:
  ```bash
  deskbrid proxmox setup-ssh-key  # generates ~/.config/deskbrid/proxmox_exec_id_ed25519
                                  # prints the public key for Proxmox host's authorized_keys
  ```
- **Command restriction (hardened setup)** — Restrict the dedicated key to *only* run
  `pct exec` and `qm guest exec` using SSH's `command=` directive in `authorized_keys`.
  Even if the key is stolen, it can't be used for arbitrary shell access, port forwarding,
  or interactive logins:

  ```authorized_keys
  command="case \"$SSH_ORIGINAL_COMMAND\" in
    \"pct exec\"*) exec $SSH_ORIGINAL_COMMAND ;;
    \"qm guest exec\"*) exec $SSH_ORIGINAL_COMMAND ;;
    *) echo 'Unauthorized command' ; exit 1 ;;
  esac",no-pty,no-agent-forwarding,no-port-forwarding,no-X11-forwarding ssh-ed25519 AAAA...
  ```

  **Restrictions in play:**
  - `command=...` — only the named commands pass through; anything else is rejected
  - `no-pty` — no interactive terminal sessions
  - `no-agent-forwarding` — can't piggyback on the user's SSH agent
  - `no-port-forwarding` — no tunneling
  - `no-X11-forwarding` — no X11 sessions

  This is the difference between a *secure* setup and a *hardened* one — optional but
  documented so Tuck can choose the level he wants.

---

## Part 6: Priority Order

### Phase 1: Foundation (2-3 days)

**Goal:** Read-only cluster awareness. Tuck asks "what's running?" and deskbrid answers.

1. **`src/proxmox/mod.rs`** — `ProxmoxClient` struct with `reqwest::Client`, auth, config loading
2. **`src/proxmox/auth.rs`** — API token auth (primary), ticket auth (fallback)
3. **`ProxmoxStatus` action** — wraps `/cluster/resources` — one call returns everything
4. **`ProxmoxNodes` + `ProxmoxNodeStatus`** — node-level detail
5. **`ProxmoxGuests` + `ProxmoxGuestStatus` + `ProxmoxGuestConfig`** — per-guest drilldown
6. **`ProxmoxStorageList` + `ProxmoxStorageGet`** — storage awareness
7. **`ProxmoxVersion`** — compatibility check
8. Wire into parse module, dispatch, MCP tools
9. Config loading from deskbrid TOML + env vars

**Files touched:**
- `src/proxmox/{mod,auth,types,endpoints,ssh}.rs` (new)
- `src/protocol/mod.rs` (add Action variants)
- `src/protocol/parse/proxmox.rs` (new)
- `src/protocol/serialize/proxmox.rs` (new)
- `src/daemon/proxmox.rs` (new, dispatch handler)
- `src/daemon/execute.rs` (route proxmox actions)
- `src/daemon/dispatch.rs` (proxmox check, like system/terminal checks)
- `src/mcp/tool_list.rs` (add tools)
- `src/mcp/tools.rs` (add call_tool arms)
- `Cargo.toml` (no new deps needed)
- Config file + env var reading (SSH config included) in `DaemonState`

### Phase 2: Lifecycle (2-3 days)

**Goal:** Tuck can start/stop/migrate guests.

1. **`ProxmoxGuestAction`** — start, stop, reboot, shutdown, reset, suspend, resume
2. **`ProxmoxGuestExec`** — run commands inside containers via SSH + pct exec (★ key for Tuck)
3. **`ProxmoxGuestMigrate`** — live migration between nodes
4. **Task tracking** — return UPID, optionally poll for completion
5. MCP tools for lifecycle
6. Permission analysis — these are destructive, require explicit allow

### Phase 3: Admin (3-4 days)

**Goal:** Tuck can create, destroy, resize, snapshot, and backup.

1. **`ProxmoxGuestCreate` + `ProxmoxGuestClone` + `ProxmoxGuestDelete`**
2. **`ProxmoxGuestResize` + `ProxmoxGuestConfigSet`**
3. **Snapshot CRUD** (`ProxmoxSnapshot*`)
4. **Backup management** (`ProxmoxBackup*`)
5. **`ProxmoxDisks`, `ProxmoxNetwork`, `ProxmoxDns`** — host-level ops
6. MCP tools for admin

### Phase 4: Advanced (2-3 days, stretch)

**Goal:** Networking, security, and HA awareness.

1. **`ProxmoxHA*`** — HA resource status and control
2. **`ProxmoxFirewall*`** — firewall rule management
3. **`ProxmoxSDN*`** — SDN overview
4. **`ProxmoxAptList` + update trigger** — node package management
5. **`ProxmoxJournal`** — node journal queries
6. **Smart `proxmox.status` output** — structured summary optimized for Tuck's context window

---

## Part 7: Async Task Pattern

Proxmox mutating operations return a UPID (unique task ID) immediately. The actual operation runs asynchronously. Deskbrid should:

1. **Return UPID in response** — Tuck knows a task is in-flight
2. **Provide `proxmox.task_wait` action** — polls `GET /nodes/{node}/tasks/{upid}/status` until done
3. **Optionally auto-poll** — add `wait: true` parameter to lifecycle actions, with a configurable timeout

```json
// Request
{"type": "proxmox.guest.start", "node": "pve", "vmid": 100, "guest_type": "lxc"}

// Response
{
  "ok": true,
  "task": "UPID:pve:00005F8B:...",
  "message": "Container start initiated"
}
```

```json
// Follow-up
{"type": "proxmox.task.wait", "node": "pve", "upid": "UPID:pve:00005F8B:..."}

// Response
{"ok": true, "status": "stopped", "exitstatus": "OK"}
```

---

## Part 8: Security Considerations

1. **Least privilege tokens** — Don't use `root@pam!`. Create a `deskbrid@pve!` token with only the permissions needed (e.g., `Sys.Audit`, `VM.Audit`, `VM.PowerMgmt`, `Datastore.Audit`, `Datastore.AllocateSpace`)
2. **Deskbrid permission system** — New `proxmox.*` actions go through the existing `permissions.check(peer_uid, &action)` system. Map Proxmox operations to deskbrid's severity levels
3. **Self-signed certs** — Proxmox ships with self-signed certs. `accept_invalid_certs` is necessary but should default to `false` — force users to explicitly opt in
4. **Config file permissions** — Token secrets in the config file should be protected. Deskbrid should warn if the config is world-readable
5. **No token in logs** — Audit logging must redact `Authorization` headers and token secrets

---

## Part 9: Design Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| HTTP client | `reqwest` directly | Already a dep. No immature crate risk. Full control. |
| Auth method | API tokens (primary), tickets (fallback) | Tokens are stateless, least-privilege. Easy to revoke. |
| Guest identification | `(node, vmid, type)` tuple | Matches Proxmox's identity model exactly |
| Task tracking | Return UPID, let Tuck poll | Don't block the socket on long operations |
| Config | TOML + env vars | Matches deskbrid's existing config pattern |
| Feature gate | No feature gate | Proxmox support is lightweight (no native deps), always compile it |
| MCP integration | Same pattern as a11y | `proxmox_status` tool → `ProxmoxStatus` action → `/cluster/resources` |

---

## Appendix: Quick Reference — Key API Calls

```bash
# What's everything doing right now?
GET /api2/json/cluster/resources

# What containers are on node "pve"?
GET /api2/json/nodes/pve/lxc

# Start container 100 on node "pve"
POST /api2/json/nodes/pve/lxc/100/status/start

# Get container 100's config
GET /api2/json/nodes/pve/lxc/100/config

# How much storage is free?
GET /api2/json/storage

# What's the node's CPU/mem look like?
GET /api2/json/nodes/pve/status

# Get a free VMID for next creation
GET /api2/json/cluster/nextid

# Create an LXC
POST /api2/json/nodes/pve/lxc \
  -d vmid=200 -d hostname=myapp -d storage=local-lvm \
  -d ostemplate=local:vztmpl/debian-12-standard.tar.gz \
  -d memory=2048 -d cores=2 -d net0='name=eth0,bridge=vmbr0,ip=dhcp'

# Wait for a task
GET /api2/json/nodes/pve/tasks/UPID:pve:00005F8B:.../status
```
