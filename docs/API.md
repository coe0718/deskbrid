     1|# Deskbrid API Reference
     2|
     3|Every desktop function is exposed through the NDJSON protocol. Clients send action messages to the Unix socket and receive response messages. This reference covers every action, its parameters, response format, and a real request/response example.
     4|
     5|## Protocol Basics
     6|
     7|**Transport:** Unix domain socket at `$XDG_RUNTIME_DIR/deskbrid.sock`
     8|**Encoding:** NDJSON — one JSON object per line, terminated by `\n`
     9|**Sequence numbers:** Each client message increments a per-connection `seq` counter. The daemon echoes `seq` back in the response for correlation.
    10|
    11|### Common Envelope
    12|
    13|**Response (success):**
    14|```json
    15|{"type": "response", "id": "action", "seq": 1, "status": "ok", "data": { ... }}
    16|```
    17|
    18|**Response (error):**
    19|```json
    20|{"type": "response", "id": "action", "seq": 1, "status": "error", "error": {"code": "INTERNAL_ERROR", "message": "..."}}
    21|```
    22|
    23|**Error codes:**
    24|| Code | Meaning |
    25||------|---------|
    26|| `INVALID_PARAMS` | Malformed JSON or unknown action type |
    27|| `INTERNAL_ERROR` | Backend operation failed |
    28|| `NOT_SUPPORTED` | No desktop backend loaded |
    29|
    30|### Connection Handshake
    31|
    32|On socket connect, the daemon immediately sends a `connected` message. Clients **must** wait for this before sending commands:
    33|
    34|```json
    35|{"type":"connected","id":"server","seq":0,"data":{"version":"0.4.1","protocol":"deskbrid-v2"}}
    36|```
    37|
    38|---
    39|
    40|## Windows
    41|
    42|### `windows.list`
    43|
    44|List all open windows.
    45|
    46|**Request:**
    47|```json
    48|{"type":"windows.list","id":"windows.list","seq":1}
    49|```
    50|
    51|**Response:**
    52|```json
    53|{
    54|  "type": "response",
    55|  "id": "action",
    56|  "seq": 1,
    57|  "status": "ok",
    58|  "data": [
    59|    {
    60|      "id": "0x1a00003",
    61|      "title": "Terminal",
    62|      "app_id": "org.gnome.Terminal",
    63|      "workspace_id": 0,
    64|      "is_focused": true,
    65|      "is_minimized": false,
    66|      "geometry": {"x": 0, "y": 0, "width": 1920, "height": 1080},
    67|      "pid": 1234
    68|    }
    69|  ]
    70|}
    71|```
    72|
    73|| Field | Type | Description |
    74||-------|------|-------------|
    75|| `id` | string | Window XID (hex string) |
    76|| `title` | string | Window title |
    77|| `app_id` | string | Application ID (WM class) |
    78|| `workspace_id` | number | Workspace index (0-based) |
    79|| `is_focused` | boolean | Whether this window currently has focus |
    80|| `is_minimized` | boolean | Whether the window is minimized |
    81|| `geometry` | object{?} | `{x, y, width, height}` — present when available |
    82|| `pid` | number{?} | Process ID — `null` when unavailable |
    83|
    84|**Backend:** GNOME Shell extension → `ListWindows()`
    85|
    86|---
    87|
    88|### `windows.focus`
    89|
    90|Focus a window by one or more matching criteria.
    91|
    92|**Request:**
    93|```json
    94|{"type":"windows.focus","window_id":"0x1a00003","id":"windows.focus","seq":2}
    95|```
    96|
    97|**Response:**
    98|```json
    99|{"type":"response","id":"action","seq":2,"status":"ok","data":{"focused":"0x1a00003"}}
   100|```
   101|
   102|| Param | Type | Description |
   103||-------|------|-------------|
   104|| `window_id` | string | Window ID to focus |
   105|
   106|The GNOME extension's `FocusWindow` method supports matching by `app_id` or `title` with optional case-insensitive substring or exact match. The daemon currently dispatches by raw window ID.
   107|
   108|**Backend:** GNOME Shell extension → `FocusWindow(app_id, title, exact)`
   109|
   110|---
   111|
   112|### `windows.get`
   113|
   114|Get information about a single window by ID.
   115|
   116|**Request:**
   117|```json
   118|{"type":"windows.get","window_id":"0x1a00003","id":"windows.get","seq":3}
   119|```
   120|
   121|**Response:** Same per-window format as `windows.list` data items.
   122|
   123|**Backend:** GNOME Shell extension → filters `ListWindows` result by ID.
   124|
   125|---
   126|
   127|## Workspaces
   128|
   129|### `workspaces.list`
   130|
   131|List all workspaces (virtual desktops).
   132|
   133|**Request:**
   134|```json
   135|{"type":"workspaces.list","id":"workspaces.list","seq":4}
   136|```
   137|
   138|**Response:**
   139|```json
   140|{
   141|  "type": "response",
   142|  "id": "action",
   143|  "seq": 4,
   144|  "status": "ok",
   145|  "data": [
   146|    {"id": 0, "name": "Workspace 1", "is_active": true},
   147|    {"id": 1, "name": "Workspace 2", "is_active": false}
   148|  ]
   149|}
   150|```
   151|
   152|**Backend:** GNOME Shell extension → `WorkspacesList` / `ActiveWorkspace` via `ext_call_parsed`.
   153|
   154|---
   155|
   156|### `workspaces.switch`
   157|
   158|Switch to a specific workspace by index.
   159|
   160|**Request:**
   161|```json
   162|{"type":"workspaces.switch","workspace_id":2,"id":"workspaces.switch","seq":5}
   163|```
   164|
   165|**Response:**
   166|```json
   167|{"type":"response","id":"action","seq":5,"status":"ok","data":{"workspace":2}}
   168|```
   169|
   170|| Param | Type | Description |
   171||-------|------|-------------|
   172|| `workspace_id` | number | Workspace index to activate |
   173|
   174|**Backend:** GNOME Shell extension → `ext_call_parsed("SwitchWorkspace", workspace_id)`. Uses the extension's DBus method — no Eval, no blocking calls.
   175|
   176|---
   177|
   178|### `workspaces.move_window`
   179|
   180|Move a window to a workspace, optionally following it.
   181|
   182|**Request:**
   183|```json
   184|{"type":"workspaces.move_window","window_id":"0x1a00003","workspace_id":2,"follow":true,"id":"workspaces.move_window","seq":6}
   185|```
   186|
   187|**Response:**
   188|```json
   189|{"type":"response","id":"action","seq":6,"status":"ok","data":{"moved":true}}
   190|```
   191|
   192|| Param | Type | Default | Description |
   193||-------|------|---------|-------------|
   194|| `window_id` | string | — | Window ID to move |
   195|| `workspace_id` | number | — | Target workspace index |
   196|| `follow` | boolean | `false` | Whether to also switch to the target workspace |
   197|
   198|**Backend:** GNOME Shell extension → `ext_call_parsed("MoveWindowToWorkspace", window_id, workspace_id)`. Uses the extension's DBus method — no Eval, no blocking calls.
   199|
   200|---
   201|
   202|## Input
   203|
   204|### `input.keyboard` (type text)
   205|
   206|Type text into the currently focused window.
   207|
   208|**Request:**
   209|```json
   210|{"type":"input.keyboard","action":"type","text":"Hello, world!\n","id":"input.keyboard","seq":7}
   211|```
   212|
   213|**Response:**
   214|```json
   215|{"type":"response","id":"action","seq":7,"status":"ok","data":{"typed":14}}
   216|```
   217|
   218|| Sub-action | Param | Type | Description |
   219||-----------|-------|------|-------------|
   220|| `type` | `text` | string | Text to type. Supports `\n`, `\t` escape sequences |
   221|
   222|`data.typed` reports the number of characters typed.
   223|
   224|**Backend:** Mutter RemoteDesktop API → `NotifyKeyboardKeysym`.
   225|
   226|---
   227|
   228|### `input.keyboard` (send key press)
   229|
   230|Press and release a single named key.
   231|
   232|**Request:**
   233|```json
   234|{"type":"input.keyboard","action":"key","key":"Return","id":"input.keyboard","seq":8}
   235|```
   236|
   237|**Response:**
   238|```json
   239|{"type":"response","id":"action","seq":8,"status":"ok","data":{"key":"Return"}}
   240|```
   241|
   242|| Sub-action | Param | Type | Description |
   243||-----------|-------|------|-------------|
   244|| `key` | `key` | string | Named key (e.g. `Return`, `Escape`, `Tab`, `BackSpace`) |
   245|
   246|**Backend:** Mutter RemoteDesktop API → `NotifyKeyboardKeysym`.
   247|
   248|---
   249|
   250|### `input.keyboard` (key combo)
   251|
   252|Press multiple keys simultaneously (like `Ctrl+C`).
   253|
   254|**Request:**
   255|```json
   256|{"type":"input.keyboard","action":"combo","keys":["ctrl","c"],"id":"input.keyboard","seq":9}
   257|```
   258|
   259|**Response:**
   260|```json
   261|{"type":"response","id":"action","seq":9,"status":"ok","data":{"combo":["ctrl","c"]}}
   262|```
   263|
   264|| Sub-action | Param | Type | Description |
   265||-----------|-------|------|-------------|
   266|| `combo` | `keys` | array of strings | Ordered list of keys to press simultaneously |
   267|
   268|**Backend:** Mutter RemoteDesktop API → `NotifyKeyboardKeysym` with modifier mask.
   269|
   270|---
   271|
   272|### `input.mouse` (move)
   273|
   274|Move the mouse cursor to absolute coordinates.
   275|
   276|**Request:**
   277|```json
   278|{"type":"input.mouse","action":"move","x":500.0,"y":300.0,"id":"input.mouse","seq":10}
   279|```
   280|
   281|**Response:**
   282|```json
   283|{"type":"response","id":"action","seq":10,"status":"ok","data":{"mouse":"move"}}
   284|```
   285|
   286|| Param | Type | Description |
   287||-------|------|-------------|
   288|| `action` | string | Must be `"move"` |
   289|| `x` | number | Absolute X coordinate |
   290|| `y` | number | Absolute Y coordinate |
   291|
   292|**Backend:** Mutter RemoteDesktop API → `NotifyPointerMotion` (relative) or `NotifyPointerMotionAbsolute` (requires ScreenCast).
   293|
   294|---
   295|
   296|### `input.mouse` (click)
   297|
   298|Click a mouse button at the current cursor position.
   299|
   300|**Request:**
   301|```json
   302|{"type":"input.mouse","action":"click","button":"left","id":"input.mouse","seq":11}
   303|```
   304|
   305|**Response:**
   306|```json
   307|{"type":"response","id":"action","seq":11,"status":"ok","data":{"mouse":"click"}}
   308|```
   309|
   310|| Param | Type | Default | Description |
   311||-------|------|---------|-------------|
   312|| `action` | string | — | Must be `"click"` |
   313|| `button` | string | `"left"` | `"left"`, `"right"`, `"middle"` |
   314|
   315|**Backend:** Mutter RemoteDesktop API → `NotifyPointerButton`.
   316|
   317|---
   318|
   319|### `input.mouse` (scroll)
   320|
   321|Scroll the mouse wheel.
   322|
   323|**Request:**
   324|```json
   325|{"type":"input.mouse","action":"scroll","dx":0.0,"dy":-5.0,"id":"input.mouse","seq":12}
   326|```
   327|
   328|**Response:**
   329|```json
   330|{"type":"response","id":"action","seq":12,"status":"ok","data":{"mouse":"scroll"}}
   331|```
   332|
   333|| Param | Type | Default | Description |
   334||-------|------|---------|-------------|
   335|| `action` | string | — | Must be `"scroll"` |
   336|| `dx` | number | `0.0` | Horizontal scroll amount (positive = right) |
   337|| `dy` | number | `0.0` | Vertical scroll amount (positive = down, negative = up) |
   338|
   339|**Backend:** Mutter RemoteDesktop API → `NotifyPointerAxis`.
   340|
   341|---
   342|
   343|## Clipboard
   344|
   345|### `clipboard.read`
   346|
   347|Read the current clipboard contents.
   348|
   349|**Request:**
   350|```json
   351|{"type":"clipboard.read","id":"clipboard.read","seq":13}
   352|```
   353|
   354|**Response:**
   355|```json
   356|{"type":"response","id":"action","seq":13,"status":"ok","data":{"text":"current clipboard content"}}
   357|```
   358|
   359|| Response field | Type | Description |
   360||---------------|------|-------------|
   361|| `text` | string | Plain text clipboard content |
   362|
   363|**Backend:** `wl-paste`.
   364|
   365|---
   366|
   367|### `clipboard.write`
   368|
   369|Write text to the clipboard.
   370|
   371|**Request:**
   372|```json
   373|{"type":"clipboard.write","text":"new content","id":"clipboard.write","seq":14}
   374|```
   375|
   376|**Response:**
   377|```json
   378|{"type":"response","id":"action","seq":14,"status":"ok","data":{"written":true}}
   379|```
   380|
   381|| Param | Type | Description |
   382||-------|------|-------------|
   383|| `text` | string | Text to set as clipboard content |
   384|
   385|**Backend:** `wl-copy`.
   386|
   387|---
   388|
   389|## Screenshot
   390|
   391|### `screenshot`
   392|
   393|Capture a screenshot.
   394|
   395|**Request (full screen):**
   396|```json
   397|{"type":"screenshot","id":"screenshot","seq":15}
   398|```
   399|
   400|**Request (specific monitor):**
   401|```json
   402|{"type":"screenshot","monitor":0,"id":"screenshot","seq":16}
   403|```
   404|
   405|**Request (region selection via slurp):**
   406|```json
   407|{"type":"screenshot","region":{"x":100,"y":100,"width":800,"height":600},"id":"screenshot","seq":17}
   408|```
   409|
   410|**Request (focused window):**
   411|```json
   412|{"type":"screenshot","window_id":"0x1a00003","id":"screenshot","seq":18}
   413|```
   414|
   415|**Response:**
   416|```json
   417|{
   418|  "type": "response",
   419|  "id": "action",
   420|  "seq": 15,
   421|  "status": "ok",
   422|  "data": {
   423|    "path": "/tmp/deskbrid-screenshot-1715000000.png",
   424|    "width": 1920,
   425|    "height": 1080,
   426|    "format": "png"
   427|  }
   428|}
   429|```
   430|
   431|| Param | Type | Description |
   432||-------|------|-------------|
   433|| `monitor` | number{?} | Monitor index to capture (omit for all monitors) |
   434|| `region` | object{?} | `{x, y, width, height}` in pixels |
   435|| `window_id` | string{?} | Window ID to capture (via backend-specific tooling — grim on GNOME/Hyprland, spectacle + ImageMagick on KDE) |
   436|
   437|**Response fields:**
   438|
   439|| Field | Type | Description |
   440||-------|------|-------------|
   441|| `path` | string | Absolute path to the saved PNG file |
   442|| `width` | number | Image width in pixels |
   443|| `height` | number | Image height in pixels |
   444|| `format` | string | Always `"png"` |
   445|
   446|**Backend:** `grim` (GNOME/Hyprland) or `spectacle` + ImageMagick `convert -crop` (KDE). Screenshots are saved to `/tmp/deskbrid-screenshot-<unix_timestamp>.png`.
   447|
   448|---
   449|
   450|## Notifications
   451|
   452|### `notification.send`
   453|
   454|Send a desktop notification.
   455|
   456|**Request:**
   457|```json
   458|{
   459|  "type": "notification.send",
   460|  "app_name": "deskbrid",
   461|  "title": "Download Complete",
   462|  "body": "Your file has finished downloading.",
   463|  "urgency": "normal",
   464|  "id": "notification.send",
   465|  "seq": 19
   466|}
   467|```
   468|
   469|**Response:**
   470|```json
   471|{"type":"response","id":"action","seq":19,"status":"ok","data":{"notification_id":42}}
   472|```
   473|
   474|| Param | Type | Default | Description |
   475||-------|------|---------|-------------|
   476|| `app_name` | string | `"deskbrid"` | Application name shown in notification |
   477|| `title` | string | — | Notification title |
   478|| `body` | string | `""` | Notification body text |
   479|| `urgency` | string | `"normal"` | `"low"`, `"normal"`, or `"critical"` |
   480|
   481|**Backend:** `notify-send`.
   482|
   483|---
   484|
   485|### `notification.close`
   486|
   487|Close a notification by ID.
   488|
   489|**Request:**
   490|```json
   491|{"type":"notification.close","notification_id":42,"id":"notification.close","seq":20}
   492|```
   493|
   494|**Response:**
   495|```json
   496|{"type":"response","id":"action","seq":20,"status":"ok","data":{"closed":42}}
   497|```
   498|
   499|| Param | Type | Description |
   500||-------|------|-------------|
   501|