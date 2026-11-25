//! Headless deployments have no native WebView. Existing callers fall back to
//! the upstream Node sidecars when the browser job is unavailable.
use std::{collections::HashMap, time::Duration};
pub const RESULT_PATH: &str = "/webviewresult";
pub fn resolve(_query: HashMap<String, String>) {}
pub async fn run(
    _label: &str, _page: &str, _params: &[(&str, &str)], _timeout: Duration,
) -> Option<HashMap<String, String>> {
    None
}
