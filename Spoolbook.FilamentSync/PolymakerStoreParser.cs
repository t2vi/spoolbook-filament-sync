using System.Text.Json;
using System.Text.RegularExpressions;

namespace Spoolbook.FilamentSync;

public record PolymakerProduct(string Title, IReadOnlyList<string> Colors);

public static partial class PolymakerStoreParser
{
    private static readonly JsonSerializerOptions JsonOptions = new() { PropertyNameCaseInsensitive = true };

    private static readonly string[] RealFilamentTypes = ["Polymaker Filament", "Panchroma Filament", "Fiberon Filament"];

    public static IReadOnlyList<PolymakerProduct> ParseCollection(string json)
    {
        var data = JsonSerializer.Deserialize<ShopifyCollection>(json, JsonOptions);
        var result = new List<PolymakerProduct>();

        foreach (var product in data?.Products ?? [])
        {
            if (!RealFilamentTypes.Contains(product.ProductType)) continue;
            // "Panchroma PLA Refill" mixes colors from several sub-lines (Matte/Silk/Gradient/
            // Marble) into one flat list with no way to attribute a color to its real sub-line.
            if (product.Title.Contains("Refill", StringComparison.OrdinalIgnoreCase)) continue;
            // "PolyLite CosPLA"'s "Color" option is actually a formula-variant selector
            // ("Version A - Durability...", "Version B - Sand-ability..."), not real colors.
            if (product.Title.Contains("CosPLA", StringComparison.OrdinalIgnoreCase)) continue;

            var colors = product.Options?.FirstOrDefault(o => o.Name == "Color")?.Values;
            if (colors is null || colors.Count == 0) continue;

            result.Add(new PolymakerProduct(product.Title, colors));
        }

        return result;
    }

    private static readonly string[] BrandLines =
        ["Panchroma", "PolyLite", "PolyMax", "PolyFlex", "PolySmooth", "PolySonic",
         "PolyCast", "PolyDissolve", "PolySupport", "PolyMide", "Polymaker"];

    // Longest/most-specific first so e.g. "HT-PLA-GF" matches before "PLA".
    private static readonly string[] KnownMaterials =
        ["HT-PLA-GF", "HT-PLA", "LW-PLA", "PC-ABS", "PC-FR", "PLA-CF",
         "TPU95-HF", "TPU90", "TPU95", "PLA", "ABS", "PETG", "PET", "PC", "ASA", "PVA", "CoPA", "CoPE"];

    // A few product names don't carry their real material as a plain word in the title at all
    // (marketing names like "PolyCast", "PolySmooth") or need a materials-list entry that would
    // otherwise collide with generic tokens ("PA12") — explicit rather than guessed.
    private static readonly Dictionary<string, (string Material, string? Variant)> Overrides = new(StringComparer.OrdinalIgnoreCase)
    {
        ["PolyCast"] = ("PLA", "PolyCast"),
        ["PolySmooth"] = ("PVB", "PolySmooth"),
        ["PolyDissolve S1 (PVA)"] = ("PVA", "PolyDissolve S1"),
        ["PolySupport for PA12"] = ("PA12", "PolySupport"),
        ["PolySupport for PLA"] = ("PLA", "PolySupport"),
        ["Panchroma Gradient Celestial"] = ("PLA", "Panchroma Gradient Celestial"),
        ["Panchroma Gradient Crystal"] = ("PLA", "Panchroma Gradient Crystal"),
        ["Panchroma Gradient Galaxy"] = ("PLA", "Panchroma Gradient Galaxy"),
        ["Panchroma Gradient Neon"] = ("PLA", "Panchroma Gradient Neon"),
        ["Panchroma Gradient Silk"] = ("PLA", "Panchroma Gradient Silk"),
        ["Panchroma Gradient Starlight"] = ("PLA", "Panchroma Gradient Starlight"),
    };

    [GeneratedRegex(@"\s+")]
    private static partial Regex WhitespaceRegex();

    // Some titles use U+00A0 (non-breaking space) after the trademark symbol instead of a
    // plain space — normalize both away before any matching.
    private static string Normalize(string title) =>
        WhitespaceRegex().Replace(title.Replace("™", "").Replace(' ', ' '), " ").Trim();

    private static string CleanUp(string s) => WhitespaceRegex().Replace(s, " ").Trim(' ', '-');

    public static (string Material, string? Variant) SplitMaterialVariant(string rawTitle)
    {
        var title = Normalize(rawTitle);

        if (Overrides.TryGetValue(title, out var overridden)) return overridden;
        if (title.StartsWith("Fiberon", StringComparison.OrdinalIgnoreCase))
            return (CleanUp(title["Fiberon".Length..]), null);

        var brand = BrandLines.FirstOrDefault(b => title.StartsWith(b, StringComparison.OrdinalIgnoreCase));
        var rest = brand is null ? title : title[brand.Length..].Trim();

        var material = KnownMaterials.FirstOrDefault(m => Regex.IsMatch(rest, $@"\b{Regex.Escape(m)}\b", RegexOptions.IgnoreCase));
        if (material is null) return (CleanUp(rest), brand is null ? null : CleanUp(brand));

        var remainder = CleanUp(Regex.Replace(rest, $@"\b{Regex.Escape(material)}\b", "", RegexOptions.IgnoreCase));
        var variant = CleanUp($"{brand} {remainder}");
        return (material, variant.Length == 0 ? null : variant);
    }

    private record ShopifyCollection(List<ShopifyProduct> Products);

    private record ShopifyProduct(
        string Title,
        [property: System.Text.Json.Serialization.JsonPropertyName("product_type")] string ProductType,
        List<ShopifyOption>? Options);

    private record ShopifyOption(string Name, List<string> Values);
}
