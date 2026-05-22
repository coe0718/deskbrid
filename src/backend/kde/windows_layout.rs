use super::*;

pub(super) async fn window_minimize(backend: &KdeBackend, id: &str) -> anyhow::Result<()> {
    KdeBackend::ensure_window_id(id)?;
    let js = format!(
        r#"
{}
if (target) {{
    try {{
        target.minimized = true;
        print("MINIMIZED:" + String(target.internalId));
    }} catch (e) {{
        print("ERROR:" + e);
    }}
}}
"#,
        KdeBackend::kwin_find_window_js(id)
    );
    backend
        .kwin_expect_marker(&js, "MINIMIZED:", &format!("window not found: {}", id))
        .await
}

pub(super) async fn window_maximize(backend: &KdeBackend, id: &str) -> anyhow::Result<()> {
    KdeBackend::ensure_window_id(id)?;
    let js = format!(
        r#"
{}
if (target) {{
    try {{
        var ok = false;
        if (typeof target.setMaximize === "function") {{
            target.setMaximize(true, true);
            ok = true;
        }} else {{
            if ("maximized" in target) {{
                target.maximized = true;
                ok = true;
            }}
            if ("maximizedHorizontally" in target) {{
                target.maximizedHorizontally = true;
                ok = true;
            }}
            if ("maximizedVertically" in target) {{
                target.maximizedVertically = true;
                ok = true;
            }}
        }}
        if (ok) {{
            print("MAXIMIZED:" + String(target.internalId));
        }} else {{
            print("ERROR:no maximize method available");
        }}
    }} catch (e) {{
        print("ERROR:" + e);
    }}
}}
"#,
        KdeBackend::kwin_find_window_js(id)
    );
    backend
        .kwin_expect_marker(&js, "MAXIMIZED:", &format!("window not found: {}", id))
        .await
}

pub(super) async fn window_move_resize(
    backend: &KdeBackend,
    id: &str,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
) -> anyhow::Result<()> {
    KdeBackend::ensure_window_id(id)?;
    let js = format!(
        r#"
{}
if (target) {{
    try {{
        var geom = {{x: {}, y: {}, width: {}, height: {}}};
        var ok = false;
        if (typeof target.moveResize === "function") {{
            target.moveResize({}, {}, {}, {});
            ok = true;
        }} else if ("frameGeometry" in target) {{
            target.frameGeometry = geom;
            ok = true;
        }} else if ("geometry" in target) {{
            target.geometry = geom;
            ok = true;
        }} else {{
            target.x = {};
            target.y = {};
            target.width = {};
            target.height = {};
            ok = true;
        }}
        if (ok) {{
            print("MOVED_RESIZED:" + String(target.internalId));
        }} else {{
            print("ERROR:no move/resize method available");
        }}
    }} catch (e) {{
        print("ERROR:" + e);
    }}
}}
"#,
        KdeBackend::kwin_find_window_js(id),
        x,
        y,
        width,
        height,
        x,
        y,
        width,
        height,
        x,
        y,
        width,
        height
    );
    backend
        .kwin_expect_marker(&js, "MOVED_RESIZED:", &format!("window not found: {}", id))
        .await
}
