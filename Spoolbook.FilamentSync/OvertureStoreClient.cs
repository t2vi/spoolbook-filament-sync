namespace Spoolbook.FilamentSync;

// Single request only (Shopify's products.json returns the whole catalog at once — Overture's
// storefront only sells filament, so no collection scoping is needed). overture3d.com's theme
// pages intermittently 503 (Shopify's own generic outage page, not a bot wall), but
// products.json itself has been reliable.
public class OvertureStoreClient : IDisposable
{
    private const string BaseUrl = "https://www.overture3d.com";

    private readonly HttpClient _http = new();

    public OvertureStoreClient()
    {
        _http.DefaultRequestHeaders.UserAgent.ParseAdd(
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0 Safari/537.36");
    }

    public async Task<string> FetchCollectionAsync()
    {
        var response = await _http.GetAsync($"{BaseUrl}/products.json?limit=250");
        response.EnsureSuccessStatusCode();
        return await response.Content.ReadAsStringAsync();
    }

    public void Dispose() => _http.Dispose();
}
