namespace Spoolbook.FilamentSync;

// Single request only (Shopify's products.json returns the whole collection at once) —
// no rate-limit delay loop needed, unlike Bambu/eSUN's per-product-page fetching.
public class ElegooStoreClient : IDisposable
{
    private const string BaseUrl = "https://www.elegoo.com";

    private readonly HttpClient _http = new();

    public ElegooStoreClient()
    {
        _http.DefaultRequestHeaders.UserAgent.ParseAdd(
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0 Safari/537.36");
    }

    public async Task<string> FetchCollectionAsync()
    {
        var response = await _http.GetAsync($"{BaseUrl}/collections/filaments/products.json?limit=250");
        response.EnsureSuccessStatusCode();
        return await response.Content.ReadAsStringAsync();
    }

    public void Dispose() => _http.Dispose();
}
