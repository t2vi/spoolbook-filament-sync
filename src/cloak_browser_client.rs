use chromiumoxide::Browser;
use futures_util::StreamExt;
use regex::Regex;
use serde::Deserialize;

// Stealth-browser fetch for manufacturer stores behind an anti-bot wall that blocks plain
// reqwest requests (see spoolbook's docs/adr/0012 "reversing abandon if blocked" addendum). Same
// CloakBrowser sidecar + CDP-connect pattern the old .NET client used via Playwright.
//
// ponytail: unlike the C# version's per-fetch NewContextAsync (isolated cookie jar), this opens
// a plain new page/tab per fetch with no explicit browser-context isolation — chromiumoxide's
// context API exists but nothing here reads/writes cookies or needs session isolation between
// fetches (each call is a one-shot anonymous GET), so it's unneeded ceremony. Add it back if a
// future source needs to look logged-in or keep state across requests.
pub struct CloakBrowserClient {
    ws_url: String,
}

#[derive(Deserialize)]
struct CdpVersion {
    #[serde(rename = "webSocketDebuggerUrl")]
    web_socket_debugger_url: String,
}

impl CloakBrowserClient {
    pub fn new(ws_url: String) -> Self {
        Self { ws_url }
    }

    // CloakBrowser reports its own internal hostname (e.g. 0.0.0.0) in the CDP WebSocket URL,
    // which is unreachable from outside the container. Rewrite the host to match config_url's
    // host instead — works for both local dev (localhost:3000) and CI (localhost via service
    // container port mapping).
    pub fn rewrite_cdp_host(cdp_ws_url: &str, config_url: &str) -> String {
        let host = config_url
            .split("://")
            .nth(1)
            .and_then(|rest| rest.split('/').next())
            .unwrap_or(config_url);
        let re = Regex::new(r"^ws://[^/]+").unwrap();
        re.replace(cdp_ws_url, format!("ws://{host}")).into_owned()
    }

    pub async fn fetch_page_html(&self, url: &str, timeout_ms: u64) -> Result<String, String> {
        let version_url = format!("{}/json/version", self.ws_url);
        let version: CdpVersion = reqwest::get(&version_url)
            .await
            .map_err(|e| format!("CloakBrowser /json/version fetch failed: {e}"))?
            .json()
            .await
            .map_err(|e| format!("CloakBrowser /json/version parse failed: {e}"))?;
        let ws = Self::rewrite_cdp_host(&version.web_socket_debugger_url, &self.ws_url);

        let (browser, mut handler) = Browser::connect(&ws)
            .await
            .map_err(|e| format!("CDP connect failed: {e}"))?;

        // The handler drives the CDP event loop — nothing else pumps it, so every browser/page
        // call would hang forever without this task running alongside it.
        let handler_task = tokio::spawn(async move { while handler.next().await.is_some() {} });

        let result = async {
            let page = browser
                .new_page(url)
                .await
                .map_err(|e| format!("CDP new_page failed: {e}"))?;
            let deadline = tokio::time::Duration::from_millis(timeout_ms);
            tokio::time::timeout(deadline, page.wait_for_navigation())
                .await
                .map_err(|_| "CDP navigation timed out".to_string())?
                .map_err(|e| format!("CDP navigation failed: {e}"))?;
            page.content()
                .await
                .map_err(|e| format!("CDP content read failed: {e}"))
        }
        .await;

        handler_task.abort();
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rewrite_cdp_host_replaces_internal_host_with_configured_host() {
        let result = CloakBrowserClient::rewrite_cdp_host(
            "ws://0.0.0.0:3000/devtools/browser/abc-123",
            "http://localhost:3000",
        );
        assert_eq!(result, "ws://localhost:3000/devtools/browser/abc-123");

        let result = CloakBrowserClient::rewrite_cdp_host(
            "ws://172.17.0.2:3000/devtools/browser/xyz",
            "http://cloakbrowser:3000",
        );
        assert_eq!(result, "ws://cloakbrowser:3000/devtools/browser/xyz");
    }
}
