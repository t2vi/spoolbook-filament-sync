using System.Text.Json;
using System.Text.RegularExpressions;

namespace Spoolbook.FilamentSync;

public record CrealityProduct(string Title, IReadOnlyList<string> Colors);

public static partial class CrealityStoreParser
{
    private static readonly JsonSerializerOptions JsonOptions = new() { PropertyNameCaseInsensitive = true };

    // "White*2+Black*2" and "2-Pack"/"3-Pack(Grey)" are multi-roll pack pseudo-options baked
    // into the same product's Color list, not real colors.
    [GeneratedRegex(@"\*\d+|\d+-Pack", RegexOptions.IgnoreCase)]
    private static partial Regex PackMultiplierRegex();

    // A real two-tone blend ("Golden-silver") is exactly one Capitalized word, a hyphen, then
    // one lowercase word. Fancy gradient names ("Wild Blossom-Long") have a multi-word or
    // non-color side and must not be touched.
    [GeneratedRegex(@"^[A-Z][a-z]+-[a-z]+$")]
    private static partial Regex TwoToneColorRegex();

    private static string NormalizeColor(string color) =>
        TwoToneColorRegex().IsMatch(color) ? color.Replace('-', '+') : color;

    public static IReadOnlyList<CrealityProduct> ParseCollection(string json)
    {
        var data = JsonSerializer.Deserialize<ShopifyCollection>(json, JsonOptions);
        var result = new List<CrealityProduct>();

        foreach (var product in data?.Products ?? [])
        {
            // Resin shares the same store/collection shape but isn't a filament.
            if (product.Title.Contains("Resin", StringComparison.OrdinalIgnoreCase)) continue;

            var colors = product.Options?.FirstOrDefault(o => o.Name == "Color")?.Values
                .Where(c => !PackMultiplierRegex().IsMatch(c)).Select(NormalizeColor).ToList();
            if (colors is null || colors.Count == 0) continue;

            result.Add(new CrealityProduct(product.Title, colors));
        }

        return result;
    }

    private static readonly string[] KnownMaterials = ["PETG-CF", "PLA-CF", "PLA", "PETG", "ABS", "TPU", "ASA", "PC"];

    [GeneratedRegex(@"\b\d+(\.\d+)?\s*(kg|mm)\b", RegexOptions.IgnoreCase)]
    private static partial Regex WeightOrDiameterRegex();

    [GeneratedRegex(@"\s*\d*d?\s*Print(ing|er) Filament.*$", RegexOptions.IgnoreCase)]
    private static partial Regex TrailingMarketingRegex();

    // Creality's product_type field is unreliable (mostly blank) — Material/Variant come from
    // the title instead, after stripping weight/diameter tokens and the trailing
    // "3D Printing/Printer Filament ..." marketing phrase.
    public static (string Material, string? Variant) SplitMaterialVariant(string rawTitle)
    {
        var t = TrailingMarketingRegex().Replace(rawTitle, "");
        t = WeightOrDiameterRegex().Replace(t, "");
        t = Regex.Replace(t, @"\s+", " ").Trim();

        var material = KnownMaterials.FirstOrDefault(m => Regex.IsMatch(t, $@"\b{Regex.Escape(m)}\b", RegexOptions.IgnoreCase));
        if (material is null) return (t, null);

        var variant = Regex.Replace(t, $@"\b{Regex.Escape(material)}\b", "", RegexOptions.IgnoreCase);
        variant = Regex.Replace(variant, @"\s+", " ").Trim(' ', '-');
        return (material, variant.Length == 0 ? null : variant);
    }

    private record ShopifyCollection(List<ShopifyProduct> Products);

    private record ShopifyProduct(string Title, List<ShopifyOption>? Options);

    private record ShopifyOption(string Name, List<string> Values);
}
