using System.Text.Json;
using System.Text.Json.Serialization;
using System.Text.RegularExpressions;

namespace Spoolbook.FilamentSync;

public record ElegooProduct(string Title, IReadOnlyList<string> Colors);

public static partial class ElegooStoreParser
{
    // Elegoo's own store is a stock Shopify theme, so unlike Bambu/eSUN this reads the
    // platform's public products.json endpoint directly instead of regexing HTML.
    private static readonly JsonSerializerOptions JsonOptions = new() { PropertyNameCaseInsensitive = true };

    // Single-roll products (the ones we want) never mention a weight in the title (e.g. "PLA
    // Plus", "ASA"). Bulk repackagings do ("PLA Filament 1.75mm Black 10KG", "Mini 250 g
    // Filament Bundle") — same colors as the single roll, just resold in bulk, so skip them
    // rather than create duplicate/garbage-variant catalog entries.
    [GeneratedRegex(@"\d+\s*(kg|g)\b", RegexOptions.IgnoreCase)]
    private static partial Regex BulkWeightRegex();

    private static readonly string[] KnownMaterials = ["PLA-CF", "PLA", "PETG", "ASA", "TPU", "PC"];

    public static IReadOnlyList<ElegooProduct> ParseCollection(string json)
    {
        var data = JsonSerializer.Deserialize<ShopifyCollection>(json, JsonOptions);
        var result = new List<ElegooProduct>();

        foreach (var product in data?.Products ?? [])
        {
            if (product.ProductType != "3D Filaments") continue;
            if (BulkWeightRegex().IsMatch(product.Title)) continue;

            // Some products (e.g. "PLA Matte") mix real colors with bulk-pack pseudo-options
            // ("2kg option1", "4KG-OPTION1") directly inside the same Color list.
            var colors = product.Options?.FirstOrDefault(o => o.Name == "Color")?.Values
                .Where(c => !BulkWeightRegex().IsMatch(c)).ToList();
            if (colors is null || colors.Count == 0) continue;

            result.Add(new ElegooProduct(product.Title, colors));
        }

        return result;
    }

    // Matches a known material as a whole word (not substring) so e.g. "PLA-CF" and "PLA"
    // never collide; whatever prefix/suffix words remain (e.g. "Rapid" + "Plus") become the Variant.
    public static (string Material, string? Variant) SplitMaterialVariant(string title)
    {
        var words = title.Split(' ');
        var materialIndex = Array.FindIndex(words, w => KnownMaterials.Contains(w, StringComparer.OrdinalIgnoreCase));
        if (materialIndex < 0) return (title, null);

        var material = words[materialIndex];
        var rest = words.Where((_, i) => i != materialIndex).ToArray();
        return (material, rest.Length == 0 ? null : string.Join(' ', rest));
    }

    private record ShopifyCollection(List<ShopifyProduct> Products);

    private record ShopifyProduct(
        string Title,
        [property: JsonPropertyName("product_type")] string ProductType,
        List<ShopifyOption>? Options);

    private record ShopifyOption(string Name, List<string> Values);
}
