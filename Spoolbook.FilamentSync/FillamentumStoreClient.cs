namespace Spoolbook.FilamentSync;

// Fillamentum's own site (fillamentum.com) is WordPress/WooCommerce with no product-data API;
// the actual store lives on the Shopify-backed shop.fillamentum.com subdomain instead.
public class FillamentumStoreClient : IDisposable
{
    private const string BaseUrl = "https://shop.fillamentum.com";

    private readonly HttpClient _http = new();

    public FillamentumStoreClient()
    {
        _http.DefaultRequestHeaders.UserAgent.ParseAdd(
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0 Safari/537.36");
    }

    public async Task<List<FillamentumProduct>> FetchAllProductsAsync()
    {
        var result = new List<FillamentumProduct>();

        for (var page = 1; page <= 10; page++)
        {
            var response = await _http.GetAsync($"{BaseUrl}/products.json?limit=250&page={page}");
            response.EnsureSuccessStatusCode();
            var json = await response.Content.ReadAsStringAsync();

            var pageProducts = FillamentumStoreParser.ParseCollection(json);
            if (pageProducts.Count == 0) break;

            result.AddRange(pageProducts);
        }

        return result;
    }

    public void Dispose() => _http.Dispose();
}
