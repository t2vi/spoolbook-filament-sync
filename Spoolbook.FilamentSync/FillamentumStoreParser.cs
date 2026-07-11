using System.Text.Json;
using System.Text.Json.Serialization;
using System.Text.RegularExpressions;

namespace Spoolbook.FilamentSync;

public record FillamentumProduct(string Title, IReadOnlyList<string> VariantTitles);

public static partial class FillamentumStoreParser
{
    private static readonly JsonSerializerOptions JsonOptions = new() { PropertyNameCaseInsensitive = true };

    // "15 m Sample"/"6 m Sample"/"Swatch"/"Sampler" listings re-sell small cuts of colors that
    // already exist as full-size products under the same material line — real colors, but
    // duplicate ones, so skip the whole listing rather than double-counting.
    [GeneratedRegex(@"^\d+\s*m sample\b|^swatch\b|^sampler\b", RegexOptions.IgnoreCase)]
    private static partial Regex SampleListingRegex();

    public static IReadOnlyList<FillamentumProduct> ParseCollection(string json)
    {
        var data = JsonSerializer.Deserialize<ShopifyCollection>(json, JsonOptions);
        var result = new List<FillamentumProduct>();

        foreach (var product in data?.Products ?? [])
        {
            if (product.ProductType != "filament for 3D printing") continue;
            if (SampleListingRegex().IsMatch(product.Title)) continue;
            // "+ LockPAd" bundles a spool-lock accessory onto colors already listed as their
            // own standalone products (e.g. "Nylon FX256 \"Sky Blue\"") — pure duplicate noise.
            if (product.Title.Contains("LockPAd", StringComparison.OrdinalIgnoreCase)) continue;

            var variantTitles = product.Variants?.Select(v => v.Title).ToList() ?? [];
            result.Add(new FillamentumProduct(product.Title, variantTitles));
        }

        return result;
    }

    // Ordered longest-prefix-first: base product-line text (title up to the quoted color, or
    // the whole title when there's no quote) maps to a canonical (Material, Variant) pair
    // matching the app's existing material vocabulary (e.g. "Nylon" not "PA", "PP" not
    // "Polypropylene", "rPETG" not "rePETG").
    private static readonly (string Prefix, string Material, string? Variant)[] MaterialMap =
    [
        ("ABS Extrafill", "ABS", "Extrafill"),
        ("ASA CF10 Carbon", "ASA", "CF10 Carbon"),
        ("ASA Extrafill", "ASA", "Extrafill"),
        ("CPE CF112 Carbon", "CPE", "CF112 Carbon"),
        ("CPE HG100", "CPE", "HG100"),
        ("Flexfill TPE 90A", "TPE", "Flexfill 90A"),
        ("Flexfill TPU 92A", "TPU", "Flexfill 92A"),
        ("Flexfill TPU 98A", "TPU", "Flexfill 98A"),
        ("HIPS Extrafill", "HIPS", "Extrafill"),
        ("NonOilen®", "NonOilen®", null),
        ("Nylon AF80 Aramid", "Nylon", "AF80 Aramid"),
        ("Nylon CF15 Carbon", "Nylon", "CF15 Carbon"),
        ("Nylon FX256", "Nylon", "FX256"),
        ("0rCA®", "Nylon", "0rCA®"),
        ("PETG", "PETG", null),
        ("PLA Crystal Clear", "PLA", "Crystal Clear"),
        ("PLA Extrafill", "PLA", "Extrafill"),
        ("Polypropylene PP 2320", "PP", "2320"),
        ("Timberfill®", "PLA", "Timberfill®"),
        ("Vinyl 303", "Vinyl", "303"),
        ("rePETG Loopfill", "rPETG", "Loopfill"),
    ];

    private static (string Material, string? Variant) SplitMaterialVariant(string baseLine)
    {
        foreach (var (prefix, material, variant) in MaterialMap.OrderByDescending(m => m.Prefix.Length))
            if (baseLine.StartsWith(prefix, StringComparison.Ordinal))
                return (material, variant);

        return (baseLine, null);
    }

    [GeneratedRegex(@"^\d+(\.\d+)?\s*(mm|kg|g)$", RegexOptions.IgnoreCase)]
    private static partial Regex WeightOrDiameterTokenRegex();

    public static (string Material, string? Variant, IReadOnlyList<string> Colors) ParseProduct(FillamentumProduct product)
    {
        var quoteMatch = Regex.Match(product.Title, "\"([^\"]+)\"");

        if (quoteMatch.Success)
        {
            var baseLine = product.Title[..quoteMatch.Index].TrimEnd('|', ' ');
            var (material, variant) = SplitMaterialVariant(baseLine);
            var color = quoteMatch.Groups[1].Value;
            // "Custom Color" is a pick-your-own-color listing, not a real color.
            var colors = color.Equals("Custom Color", StringComparison.OrdinalIgnoreCase)
                ? []
                : (IReadOnlyList<string>)[color];
            return (material, variant, colors);
        }

        var noQuoteBase = Regex.Replace(product.Title, @"\s*\|.*$", "").Trim();
        var (mat, var) = SplitMaterialVariant(noQuoteBase);

        var extractedColors = product.VariantTitles
            .Select(t => t.Split('/').Select(s => s.Trim()).LastOrDefault() ?? "")
            .Where(last => last.Length > 0
                && !last.Equals("Default Title", StringComparison.OrdinalIgnoreCase)
                && !WeightOrDiameterTokenRegex().IsMatch(last))
            .Distinct()
            .OrderBy(c => c, StringComparer.Ordinal)
            .ToList();

        IReadOnlyList<string> colorsOut = extractedColors.Count > 0 ? extractedColors : ["Natural"];
        return (mat, var, colorsOut);
    }

    private record ShopifyCollection(List<ShopifyProduct> Products);

    private record ShopifyProduct(
        string Title,
        [property: JsonPropertyName("product_type")] string ProductType,
        List<ShopifyVariant>? Variants);

    private record ShopifyVariant(string Title);
}
