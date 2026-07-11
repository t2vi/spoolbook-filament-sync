using System.Globalization;
using System.Text.Json;
using System.Text.Json.Serialization;

namespace Spoolbook.FilamentSync;

public record OvertureProduct(string ProductType, IReadOnlyList<string> Colors);

public static class OvertureStoreParser
{
    private static readonly JsonSerializerOptions JsonOptions = new() { PropertyNameCaseInsensitive = true };

    public static IReadOnlyList<OvertureProduct> ParseCollection(string json)
    {
        var data = JsonSerializer.Deserialize<ShopifyCollection>(json, JsonOptions);
        var result = new List<OvertureProduct>();

        foreach (var product in data?.Products ?? [])
        {
            if (!product.ProductType.StartsWith("3D Printer Filament", StringComparison.OrdinalIgnoreCase)) continue;

            var colors = product.Options?.FirstOrDefault(o => o.Name == "Color")?.Values;
            if (colors is null || colors.Count == 0) continue;

            result.Add(new OvertureProduct(product.ProductType, colors.Select(NormalizeColor).ToList()));
        }

        return result;
    }

    // Dual/gradient color names use a single hyphen ("Black-White") where every other scraper
    // in this codebase uses "+" ("Black+White") — normalize so these get the same multi-color
    // swatch treatment. Fancy single-word names never contain exactly one hyphen, so this is safe.
    private static string NormalizeColor(string color) =>
        color.Count(c => c == '-') == 1 ? color.Replace('-', '+') : color;

    // Overture's own product_type taxonomy already encodes Material/Variant hierarchically
    // (e.g. "3D Printer Filament > PLA > MATTE PLA") — no title parsing needed.
    public static (string Material, string? Variant) SplitMaterialVariant(string productType)
    {
        var segments = productType.Split('>').Select(s => s.Trim()).ToArray();
        var material = segments[1];
        var variantRaw = segments[2].Replace(material, "", StringComparison.OrdinalIgnoreCase).Trim(' ', '+');

        if (variantRaw.Length == 0) return (material, null);

        var variant = string.Join(' ', variantRaw.Split(' ', StringSplitOptions.RemoveEmptyEntries)
            .Select(w => CultureInfo.InvariantCulture.TextInfo.ToTitleCase(w.ToLowerInvariant())));
        return (material, variant);
    }

    private record ShopifyCollection(List<ShopifyProduct> Products);

    private record ShopifyProduct(
        [property: JsonPropertyName("product_type")] string ProductType,
        List<ShopifyOption>? Options);

    private record ShopifyOption(string Name, List<string> Values);
}
