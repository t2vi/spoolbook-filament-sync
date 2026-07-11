namespace Spoolbook.FilamentSync;

// Bambu's own store 429s after a burst of requests (observed ~15 rapid fetches).
// A fixed delay between requests plus a Retry-After-aware backoff keeps this well under that.
public class BambuStoreClient : IDisposable
{
    private const string BaseUrl = "https://au.store.bambulab.com";
    private static readonly TimeSpan RequestDelay = TimeSpan.FromSeconds(1.5);

    private readonly HttpClient _http = new();

    public BambuStoreClient()
    {
        _http.DefaultRequestHeaders.UserAgent.ParseAdd(
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0 Safari/537.36");
    }

    public Task<string> FetchListingAsync() =>
        FetchAsync($"{BaseUrl}/collections/bambu-lab-3d-printer-filament");

    public Task<string> FetchProductAsync(string slug) =>
        FetchAsync($"{BaseUrl}/en/products/{slug}");

    private async Task<string> FetchAsync(string url)
    {
        while (true)
        {
            await Task.Delay(RequestDelay);
            var response = await _http.GetAsync(url);

            if (response.StatusCode == System.Net.HttpStatusCode.TooManyRequests)
            {
                var retryAfter = response.Headers.RetryAfter?.Delta ?? TimeSpan.FromSeconds(10);
                await Task.Delay(retryAfter);
                continue;
            }

            response.EnsureSuccessStatusCode();
            return await response.Content.ReadAsStringAsync();
        }
    }

    public void Dispose() => _http.Dispose();
}
