namespace Spoolbook.FilamentSync;

// colorFabb's Magento listing page is fixed at 12 products per page (query-string page-size
// overrides are silently ignored) with ~533 products total, so this paginates through the full
// "/filaments" category rather than a single bulk request like the Shopify-backed scrapers.
public class ColorfabbStoreClient : IDisposable
{
    private const string BaseUrl = "https://colorfabb.com";
    private static readonly TimeSpan RequestDelay = TimeSpan.FromSeconds(1);

    private readonly HttpClient _http = new() { Timeout = TimeSpan.FromSeconds(30) };

    public ColorfabbStoreClient()
    {
        _http.DefaultRequestHeaders.UserAgent.ParseAdd(
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0 Safari/537.36");
    }

    public async Task<List<string>> FetchAllProductTitlesAsync()
    {
        var titles = new List<string>();
        var seen = new HashSet<string>();

        for (var page = 1; page <= 60; page++)
        {
            if (page > 1) await Task.Delay(RequestDelay);

            var response = await _http.GetAsync($"{BaseUrl}/filaments?p={page}");
            response.EnsureSuccessStatusCode();
            var html = await response.Content.ReadAsStringAsync();

            var pageTitles = ColorfabbStoreParser.ExtractProductTitles(html);
            var newCount = 0;
            foreach (var title in pageTitles)
                if (seen.Add(title))
                {
                    titles.Add(title);
                    newCount++;
                }

            Console.WriteLine($"  page {page}: {pageTitles.Count} found, {newCount} new, total {titles.Count}");
            if (newCount == 0 && page > 1) break;
        }

        return titles;
    }

    public void Dispose() => _http.Dispose();
}
