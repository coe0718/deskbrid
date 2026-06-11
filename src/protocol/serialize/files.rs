use super::Action;
use serde_json::json;

pub(super) fn serialize_files(action: &Action, id: &str) -> serde_json::Value {
    match action {
        // Files
        Action::FilesWatch {
            path,
            recursive,
            patterns,
        } => {
            let mut obj =
                json!({"type": "files.watch", "id": id, "path": path, "recursive": recursive});
            if let Some(p) = patterns {
                obj["patterns"] = json!(p);
            }
            obj
        }
        Action::FilesUnwatch { path } => {
            json!({"type": "files.unwatch", "id": id, "path": path})
        }
        Action::FilesSearch {
            pattern,
            root,
            max_results,
        } => {
            let mut obj = json!({"type": "files.search", "id": id, "pattern": pattern, "max_results": max_results});
            if let Some(r) = root {
                obj["root"] = json!(r);
            }
            obj
        }
        Action::FilesRead {
            path,
            offset,
            limit,
        } => {
            let mut obj = json!({"type": "files.read", "id": id, "path": path});
            if let Some(o) = offset {
                obj["offset"] = json!(o);
            }
            if let Some(l) = limit {
                obj["limit"] = json!(l);
            }
            obj
        }
        Action::FilesWrite {
            path,
            content,
            append,
        } => {
            json!({"type": "files.write", "id": id, "path": path, "content": content, "append": append})
        }
        Action::FilesCopy {
            source,
            destination,
        } => {
            json!({"type": "files.copy", "id": id, "source": source, "destination": destination})
        }
        Action::FilesMove {
            source,
            destination,
        } => {
            json!({"type": "files.move", "id": id, "source": source, "destination": destination})
        }
        Action::FilesDelete { path, recursive } => {
            json!({"type": "files.delete", "id": id, "path": path, "recursive": recursive})
        }
        Action::FilesMkdir { path, parents } => {
            json!({"type": "files.mkdir", "id": id, "path": path, "parents": parents})
        }
        Action::FilesList { path } => {
            json!({"type": "files.list", "id": id, "path": path})
        }
        Action::BrowserListTabs => json!({"type": "browser.list_tabs", "id": id}),
        Action::BrowserNavigate { tab_index, url } => {
            let mut obj = json!({"type": "browser.navigate", "id": id, "url": url});
            if let Some(idx) = tab_index {
                obj["tab_index"] = json!(idx);
            }
            obj
        }
        Action::BrowserEvaluate {
            tab_index,
            expression,
            await_promise,
        } => {
            let mut obj = json!({"type": "browser.evaluate", "id": id, "expression": expression, "await_promise": await_promise});
            if let Some(idx) = tab_index {
                obj["tab_index"] = json!(idx);
            }
            obj
        }
        Action::BrowserScreenshotTab { tab_index } => {
            let mut obj = json!({"type": "browser.screenshot_tab", "id": id});
            if let Some(idx) = tab_index {
                obj["tab_index"] = json!(idx);
            }
            obj
        }
        Action::BrowserClick {
            tab_index,
            selector,
        } => {
            let mut obj = json!({"type": "browser.click", "id": id, "selector": selector});
            if let Some(idx) = tab_index {
                obj["tab_index"] = json!(idx);
            }
            obj
        }
        _ => serde_json::json!({"error": "not a files action"}),
    }
}
