use crate::DaemonState;
use crate::backend::DesktopBackend;
use crate::protocol::Action;
use serde_json::Value;

pub(crate) async fn execute_browser(
    action: Action,
    _backend: &dyn DesktopBackend,
    _state: &DaemonState,
) -> anyhow::Result<Value> {
    use Action::*;
    Ok(match action {
        BrowserListTabs => crate::browser::list_tabs().await?,
        BrowserNavigate { tab_index, ref url } => crate::browser::navigate(tab_index, url).await?,
        BrowserEvaluate {
            tab_index,
            ref expression,
            await_promise,
        } => crate::browser::evaluate(tab_index, expression, await_promise).await?,
        BrowserScreenshotTab { tab_index } => crate::browser::screenshot_tab(tab_index).await?,
        BrowserClick {
            tab_index,
            ref selector,
        } => crate::browser::click(tab_index, selector).await?,

        // Accessibility (AT-SPI2)
        _ => anyhow::bail!("internal dispatch error: not a browser action"),
    })
}
