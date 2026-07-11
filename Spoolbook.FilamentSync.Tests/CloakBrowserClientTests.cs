namespace Spoolbook.FilamentSync.Tests;

public class CloakBrowserClientTests
{
    [Theory]
    [InlineData("ws://0.0.0.0:9222/devtools/browser/abc", "http://localhost:3000", "ws://localhost:3000/devtools/browser/abc")]
    [InlineData("ws://172.17.0.2:3000/devtools/browser/xyz", "http://cloakbrowser:3000", "ws://cloakbrowser:3000/devtools/browser/xyz")]
    public void RewriteCdpHost_ReplacesHostWithConfiguredHost(string cdpWsUrl, string configUrl, string expected)
    {
        Assert.Equal(expected, CloakBrowserClient.RewriteCdpHost(cdpWsUrl, configUrl));
    }
}
