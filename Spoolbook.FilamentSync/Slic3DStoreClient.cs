namespace Spoolbook.FilamentSync;

public class Slic3DStoreClient
{
    private const string Url = "https://www.jaycar.com.au/brands/slic3d";

    public async Task<List<string>> FetchAllProductTitlesAsync(CloakBrowserClient cloak)
    {
        var html = await cloak.FetchPageHtmlAsync(Url, timeoutMs: 45_000);
        return Slic3DStoreParser.ParseListingPage(html).ToList();
    }
}
