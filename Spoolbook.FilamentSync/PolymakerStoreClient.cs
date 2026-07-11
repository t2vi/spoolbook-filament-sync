namespace Spoolbook.FilamentSync;

// Single request only (Shopify's products.json returns the whole collection at once), same
// shape as ElegooStoreClient/SunluStoreClient — polymaker.com's bare domain is behind an active
// Cloudflare bot challenge (cf-mitigated: challenge), but us.polymaker.com is a plain Shopify
// storefront with no such wall.
public class PolymakerStoreClient : IDisposable
{
    private const string BaseUrl = "https://us.polymaker.com";

    private readonly HttpClient _http = new();

    public PolymakerStoreClient()
    {
        _http.DefaultRequestHeaders.UserAgent.ParseAdd(
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0 Safari/537.36");
    }

    public async Task<string> FetchCollectionAsync()
    {
        var response = await _http.GetAsync($"{BaseUrl}/collections/all/products.json?limit=250");
        response.EnsureSuccessStatusCode();
        return await response.Content.ReadAsStringAsync();
    }

    public void Dispose() => _http.Dispose();
}
