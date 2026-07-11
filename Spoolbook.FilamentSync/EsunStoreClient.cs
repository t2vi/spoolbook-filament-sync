namespace Spoolbook.FilamentSync;

// Mirrors BambuStoreClient's rate-limit-friendly shape; eSUN's own limits are unconfirmed
// so we apply the same conservative delay/backoff rather than hammering the site to find out.
public class EsunStoreClient : IDisposable
{
    private const string BaseUrl = "https://esun3dstore.com";
    private static readonly TimeSpan RequestDelay = TimeSpan.FromSeconds(1.5);

    private readonly HttpClient _http = new();

    public EsunStoreClient()
    {
        _http.DefaultRequestHeaders.UserAgent.ParseAdd(
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0 Safari/537.36");
    }

    public Task<string> FetchListingAsync(int page) =>
        FetchAsync(page <= 1
            ? $"{BaseUrl}/collections/3d-filament"
            : $"{BaseUrl}/collections/3d-filament?page={page}");

    public Task<string> FetchProductAsync(string slug) =>
        FetchAsync($"{BaseUrl}/products/{slug}");

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
