use super::*;

pub(super) async fn files_watch(
    _backend: &CosmicBackend,
    _path: &str,
    _recursive: bool,
    _patterns: Option<&[String]>,
) -> anyhow::Result<()> {
    Ok(())
}

pub(super) async fn files_unwatch(_backend: &CosmicBackend, _path: &str) -> anyhow::Result<()> {
    Ok(())
}

pub(super) async fn files_search(
    backend: &CosmicBackend,
    pattern: &str,
    _root: Option<&str>,
    max_results: u32,
) -> anyhow::Result<Vec<String>> {
    // Reuse `find` like the other backends
    let output = backend
        .sh(
            "find",
            &[".", "-iname", &format!("*{}*", pattern), "-type", "f"],
        )
        .await?;
    let results: Vec<String> = output
        .lines()
        .take(max_results as usize)
        .map(String::from)
        .collect();
    Ok(results)
}

// ─── Audio ──────────────────────────────────────────
