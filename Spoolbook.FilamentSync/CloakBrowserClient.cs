using System.Text.Json;
using System.Text.RegularExpressions;
using Microsoft.Playwright;

namespace Spoolbook.FilamentSync;

// Stealth-browser fetch for manufacturer stores behind an anti-bot wall that blocks plain
// HttpClient requests (see docs/adr/0012's "reversing abandon if blocked" addendum). Same
// CloakBrowser sidecar + CDP-connect pattern already proven in t2vi/arrgh's plugin-host
// (docs/issues/0002-cloakbrowser-cf-plugins.md) — reusing its published image rather than
// re-deriving the CDP-proxy setup.
public partial class CloakBrowserClient : IAsyncDisposable
{
    private readonly string _wsUrl;
    private readonly HttpClient _http = new();
    private IPlaywright? _playwright;
    private IBrowser? _browser;

    public CloakBrowserClient(string? wsUrl = null)
    {
        _wsUrl = wsUrl ?? Environment.GetEnvironmentVariable("CLOAKBROWSER_WS_URL")
            ?? throw new InvalidOperationException("CLOAKBROWSER_WS_URL is not set — CF-dependent scrapers will not work");
    }

    // CloakBrowser reports its own internal hostname (e.g. 0.0.0.0) in the CDP WebSocket URL,
    // which is unreachable from outside the container. Rewrite the host to match _wsUrl's host
    // instead — works for both local dev (localhost:3000) and CI (localhost via service
    // container port mapping).
    [GeneratedRegex(@"^ws://[^/]+")]
    private static partial Regex WsHostRegex();

    public static string RewriteCdpHost(string cdpWsUrl, string configUrl)
    {
        var host = new Uri(configUrl).Authority;
        return WsHostRegex().Replace(cdpWsUrl, $"ws://{host}");
    }

    private async Task<IBrowser> GetBrowserAsync()
    {
        if (_browser is { IsConnected: true }) return _browser;

        var versionJson = await _http.GetStringAsync($"{_wsUrl}/json/version");
        using var doc = JsonDocument.Parse(versionJson);
        var cdpWsUrl = doc.RootElement.GetProperty("webSocketDebuggerUrl").GetString()!;
        var wsUrl = RewriteCdpHost(cdpWsUrl, _wsUrl);

        _playwright ??= await Playwright.CreateAsync();
        _browser = await _playwright.Chromium.ConnectOverCDPAsync(wsUrl);
        return _browser;
    }

    public async Task<string> FetchPageHtmlAsync(string url, string waitUntil = "domcontentloaded", int timeoutMs = 60_000)
    {
        var browser = await GetBrowserAsync();
        var context = await browser.NewContextAsync();
        try
        {
            var page = await context.NewPageAsync();
            await page.GotoAsync(url, new PageGotoOptions
            {
                WaitUntil = Enum.Parse<WaitUntilState>(waitUntil, ignoreCase: true),
                Timeout = timeoutMs
            });
            return await page.ContentAsync();
        }
        finally
        {
            await context.CloseAsync();
        }
    }

    public async ValueTask DisposeAsync()
    {
        if (_browser is not null) await _browser.CloseAsync();
        _playwright?.Dispose();
        _http.Dispose();
    }
}
