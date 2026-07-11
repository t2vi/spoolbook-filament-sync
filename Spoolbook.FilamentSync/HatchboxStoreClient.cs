namespace Spoolbook.FilamentSync;

// hatchbox3d.com blocks plain HttpClient requests (403 fingerprint challenge) — fetched via
// CloakBrowser instead (see docs/adr/0012's "reversing abandon if blocked" addendum).
public class HatchboxStoreClient
{
    private const string BaseUrl = "https://hatchbox3d.com";

    public async Task<List<string>> FetchAllProductTitlesAsync(CloakBrowserClient cloak)
    {
        var titles = new List<string>();
        for (var page = 1; page <= 10; page++)
        {
            var html = await cloak.FetchPageHtmlAsync($"{BaseUrl}/products.json?limit=250&page={page}");
            var pageTitles = HatchboxStoreParser.ParseCollection(html);
            if (pageTitles.Count == 0) break;
            titles.AddRange(pageTitles);
        }
        return titles;
    }
}
