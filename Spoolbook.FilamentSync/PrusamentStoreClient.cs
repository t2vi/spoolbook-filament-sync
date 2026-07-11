using System.Text;
using System.Text.Json;

namespace Spoolbook.FilamentSync;

// Prusa's shop (prusa3d.com) is a Next.js + GraphQL storefront — unlike the Shopify sites this
// needs a direct GraphQL POST rather than a products.json endpoint. Reverse-engineered via the
// server's own validation error messages (introspection is disabled) against the public,
// unauthenticated /graphql/ endpoint.
public class PrusamentStoreClient : IDisposable
{
    private const string GraphQlUrl = "https://www.prusa3d.com/graphql/";

    private const string Query = """
        query {
          category(uuid: "dbab0081-dadb-44d1-9499-19446794319c") {
            products(
              first: 300
              filter: {
                priceOptionInput: { currencyCode: "USD", vatCountryCode: "US" }
                brands: ["24824bd8-0ec7-4cc5-aea4-c3ac7c16e2d0"]
              }
            ) {
              edges { node { ... on Variant { name } } }
            }
          }
        }
        """;

    private readonly HttpClient _http = new();

    public PrusamentStoreClient()
    {
        _http.DefaultRequestHeaders.UserAgent.ParseAdd(
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0 Safari/537.36");
    }

    public async Task<string> FetchCollectionAsync()
    {
        var body = JsonSerializer.Serialize(new { query = Query });
        var response = await _http.PostAsync(GraphQlUrl, new StringContent(body, Encoding.UTF8, "application/json"));
        response.EnsureSuccessStatusCode();
        return await response.Content.ReadAsStringAsync();
    }

    public void Dispose() => _http.Dispose();
}
