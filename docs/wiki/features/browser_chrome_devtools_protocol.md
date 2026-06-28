# Browser Chrome Devtools Protocol

Control web browsers through the Chrome DevTools Protocol (CDP). This feature lets an AI agent list open browser tabs, navigate to URLs, execute JavaScript, take screenshots, and click elements — enabling automated web interaction, scraping, and testing directly through the desktop browser without requiring a separate WebDriver or headless setup.

## Actions

### browser.list_tabs

List all open tabs across detected browser instances.

| Parameter | Type | Description |
|-----------|------|-------------|
| `browser` | string | *Optional.* Target browser: `chrome`, `chromium`, `firefox`. If omitted, auto-detects a running browser. |

```bash
deskbrid browser.list_tabs '{ "browser": "chrome" }'
```

```json
{"type": "browser.list_tabs", "browser": "chrome"}
```

**Response:** Returns an array of tab objects, each with `tab_id` (string), `title` (string), `url` (string), `browser` (string), and `favicon_url` (string).

### browser.navigate

Navigate a specific browser tab to a URL.

| Parameter | Type | Description |
|-----------|------|-------------|
| `tab_id` | string | The tab ID returned by `browser.list_tabs`. |
| `url` | string | The full URL to navigate to (must include protocol, e.g. `https://example.com`). |

```bash
deskbrid browser.navigate '{ "tab_id": "tab-1", "url": "https://example.com" }'
```

```json
{"type": "browser.navigate", "tab_id": "tab-1", "url": "https://example.com"}
```

**Response:** Returns `{"success": true}` once the page has started loading. Does not wait for full page load.

### browser.evaluate

Execute arbitrary JavaScript in a browser tab and return the result.

| Parameter | Type | Description |
|-----------|------|-------------|
| `tab_id` | string | The tab ID to execute JavaScript in. |
| `expression` | string | The JavaScript expression or script to evaluate. Returns the evaluated result (serializable values only). |

```bash
deskbrid browser.evaluate '{ "tab_id": "tab-1", "expression": "document.title" }'
```

```json
{"type": "browser.evaluate", "tab_id": "tab-1", "expression": "document.title"}
```

**Response:** Returns an object with `result` containing the evaluated JavaScript value (string, number, boolean, array, or object — must be JSON-serializable). Non-serializable values (e.g. `undefined`, DOM nodes, functions) return an error or `null`.

### browser.screenshot_tab

Capture a screenshot of a browser tab's current viewport.

| Parameter | Type | Description |
|-----------|------|-------------|
| `tab_id` | string | The tab ID to screenshot. |
| `format` | string | *Optional.* Image format: `png` (default) or `jpeg`. |

```bash
deskbrid browser.screenshot_tab '{ "tab_id": "tab-1", "format": "jpeg" }'
```

```json
{"type": "browser.screenshot_tab", "tab_id": "tab-1", "format": "jpeg"}
```

**Response:** Returns a base64-encoded string of the screenshot image. Decode and save to disk for viewing.

### browser.click

Click on a DOM element in a browser tab identified by a CSS selector.

| Parameter | Type | Description |
|-----------|------|-------------|
| `tab_id` | string | The tab ID to click in. |
| `selector` | string | A CSS selector matching the target element (e.g. `"#submit-btn"`, `".nav-link"`, `"button[type=submit]"`). |

```bash
deskbrid browser.click '{ "tab_id": "tab-1", "selector": "#submit-btn" }'
```

```json
{"type": "browser.click", "tab_id": "tab-1", "selector": "#submit-btn"}
```

**Response:** Returns `{"success": true}` if the element was found and clicked. Returns an error if no element matches the selector.

## Safety Boundary

- CDP access requires the browser to be launched with a **remote debugging port** enabled (e.g. `--remote-debugging-port=9222` for Chrome/Chromium). Deskbrid does not automatically start browsers with debugging enabled — the user must configure this.
- `browser.evaluate` can execute arbitrary JavaScript, which has full access to the page's DOM, cookies, localStorage, and network. This is **privileged access** — treat it with the same caution as a browser extension.
- Firefox requires the `moz:debuggerAddress` capability and may behave differently than Chrome-based browsers.
- Tabs are sandboxed by browser origin policies — `browser.evaluate` only has access to the current page's origin, not cross-origin content.
- Confirmation mode is recommended for `browser.navigate` and `browser.click` when operating on sensitive sites.

## Local Development

1. Start Chrome/Chromium with remote debugging enabled:
   ```bash
   google-chrome --remote-debugging-port=9222 --user-data-dir=/tmp/chrome-debug
   ```
2. Ensure Deskbrid daemon is running: `deskbrid daemon`
3. List tabs:
   ```bash
   deskbrid browser.list_tabs '{ "browser": "chrome" }'
   ```
4. Navigate to a test page:
   ```bash
   deskbrid browser.navigate '{ "tab_id": "<id>", "url": "https://example.com" }'
   ```
5. Run a JS expression:
   ```bash
   deskbrid browser.evaluate '{ "tab_id": "<id>", "expression": "document.body.innerText.substring(0, 200)" }'
   ```
6. Take a screenshot:
   ```bash
   deskbrid browser.screenshot_tab '{ "tab_id": "<id>" }'
   ```

## Configuration

No Deskbrid-specific configuration is required, but the browser must be launched with a remote debugging port. Typical flags:

- **Chrome/Chromium:** `--remote-debugging-port=9222`
- **Chromium-based Edge:** `--remote-debugging-port=9222`
- **Firefox:** Requires `marionette` enabled (`about:config > marionette.enabled=true`) or use geckodriver.

Multiple browsers can run simultaneously on different debugging ports; Deskbrid auto-discovers them via CDP's discovery endpoint.
