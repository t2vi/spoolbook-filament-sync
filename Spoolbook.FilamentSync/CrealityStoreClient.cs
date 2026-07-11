namespace Spoolbook.FilamentSync;

// Single request only (Shopify's products.json returns the whole collection at once) —
// www.creality.com/store.creality.com run a custom headless storefront with no server-rendered
// product data and no standard products.json endpoint, but us.store.creality.com is a plain
// Shopify theme.
public class CrealityStoreClient : IDisposable
{
    private const string BaseUrl = "https://us.store.creality.com";

    private readonly HttpClient _http = new();

    public CrealityStoreClient()
    {
        _http.DefaultRequestHeaders.UserAgent.ParseAdd(
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0 Safari/537.36");
    }

    public async Task<string> FetchCollectionAsync()
    {
        var response = await _http.GetAsync($"{BaseUrl}/collections/materials/products.json?limit=250");
        response.EnsureSuccessStatusCode();
        return await response.Content.ReadAsStringAsync();
    }

    public void Dispose() => _http.Dispose();
}
