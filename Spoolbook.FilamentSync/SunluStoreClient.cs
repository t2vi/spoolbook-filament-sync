namespace Spoolbook.FilamentSync;

// Single request only (Shopify's products.json returns the whole collection at once), same
// shape as ElegooStoreClient — SUNLU's actual storefront is store.sunlu.com, not www.sunlu.com
// (which is a separate Nuxt marketing site with no plain-HTTP-scrapeable product data).
public class SunluStoreClient : IDisposable
{
    private const string BaseUrl = "https://store.sunlu.com";

    private readonly HttpClient _http = new();

    public SunluStoreClient()
    {
        _http.DefaultRequestHeaders.UserAgent.ParseAdd(
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0 Safari/537.36");
    }

    public async Task<string> FetchCollectionAsync()
    {
        var response = await _http.GetAsync($"{BaseUrl}/collections/3d-printer-filament/products.json?limit=250");
        response.EnsureSuccessStatusCode();
        return await response.Content.ReadAsStringAsync();
    }

    public void Dispose() => _http.Dispose();
}
