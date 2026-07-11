namespace Spoolbook.FilamentSync;

// Single request only (Shopify's products.json returns the whole collection at once) — the
// bare domain proto-pasta.com is a plain Shopify theme with no anti-bot wall.
public class ProtopastaStoreClient : IDisposable
{
    private const string BaseUrl = "https://proto-pasta.com";

    private readonly HttpClient _http = new();

    public ProtopastaStoreClient()
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
