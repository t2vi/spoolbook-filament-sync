using System.Text.Json;
using System.Text.RegularExpressions;

namespace Spoolbook.FilamentSync;

public record SunluProduct(string Title, IReadOnlyList<string> Colors);

public static partial class SunluStoreParser
{
    private static readonly JsonSerializerOptions JsonOptions = new() { PropertyNameCaseInsensitive = true };

    [GeneratedRegex(@"^\[MOQ[^\]]*\]\s*", RegexOptions.IgnoreCase)]
    private static partial Regex MoqPrefixRegex();

    // "Large Spool"/explicit 3-9kg mentions are real bulk repackagings of the same colors as
    // the 1kg listing. The negative lookbehind keeps "0.9KG/1KG" (normal single-roll fill
    // weight) from false-positiving on the "9kg" inside "0.9kg".
    [GeneratedRegex(@"large spool|clearance|special offers|(?<!\.)\b[3-9]\d*\s*kg\b", RegexOptions.IgnoreCase)]
    private static partial Regex BulkSizeRegex();

    // Used only to detect multi-material bundle titles (e.g. "PLA, SILK, PETG Rainbow...") —
    // "SILK" is deliberately excluded since SUNLU uses it both as a standalone material and as
    // a texture descriptor ("TPU-SILK"), which would false-positive real single products.
    private static readonly string[] BundleDetectMaterials = ["PLA", "PETG", "ABS", "ASA", "TPU", "PC"];

    public static IReadOnlyList<SunluProduct> ParseCollection(string json)
    {
        var data = JsonSerializer.Deserialize<ShopifyCollection>(json, JsonOptions);
        var result = new List<SunluProduct>();

        foreach (var product in data?.Products ?? [])
        {
            var title = MoqPrefixRegex().Replace(product.Title, "");
            if (BulkSizeRegex().IsMatch(title)) continue;

            var distinctMaterials = BundleDetectMaterials.Count(m => Regex.IsMatch(title, $@"\b{Regex.Escape(m)}\b", RegexOptions.IgnoreCase));
            if (distinctMaterials >= 2) continue;

            var colors = product.Options?.FirstOrDefault(o => o.Name == "Color")?.Values;
            if (colors is null || colors.Count == 0) continue;

            result.Add(new SunluProduct(title, colors.Select(CleanColorName).ToList()));
        }

        return result;
    }

    // SUNLU's own Color option values often repeat the product name (e.g. "PLA Galaxy |
    // Starlit Flow", "PETG White", "Cherry Wood 1KG") instead of just the color — strip the
    // redundant part rather than seed it verbatim.
    private static readonly string[] ColorPrefixesToStrip =
    [
        "High Speed Matte PETG", "Anti-string PLA", "High Speed PLA",
        "PLA+ 2.0", "PLA+", "PLA", "PETG", "Matte", "Silk", "Twinkling"
    ];

    private static string CleanColorName(string color)
    {
        var lastDelim = color.LastIndexOfAny(['|', '/']);
        if (lastDelim >= 0) color = color[(lastDelim + 1)..].Trim();

        foreach (var prefix in ColorPrefixesToStrip)
        {
            if (!color.StartsWith(prefix + " ", StringComparison.OrdinalIgnoreCase)) continue;
            color = color[(prefix.Length + 1)..].Trim();
            break;
        }

        if (color.EndsWith(" 1KG", StringComparison.OrdinalIgnoreCase))
            color = color[..^4].Trim();

        return color;
    }

    // SUNLU's titles are inconsistent (parenthetical aliases like "PLA+(PLA Plus)", mixed word
    // order like "Glow in The Dark (Luminous) PLA" vs "Matte PLA") — a generic token-match split
    // (like Elegoo's) produced garbage on ~40% of these, so this is an explicit lookup instead.
    private static readonly Dictionary<string, (string Material, string? Variant)> MaterialVariantByTitle = new(StringComparer.OrdinalIgnoreCase)
    {
        ["SUNLU PLA+(PLA Plus) 3D Printer Filament 1KG"] = ("PLA", "Plus"),
        ["SILK 3D Printer Filament 1KG"] = ("PLA", "Silk"),
        ["ABS 3D Printer Filament 0.9KG/1KG"] = ("ABS", null),
        ["PETG 3D Printer Filament 1KG"] = ("PETG", null),
        ["Matte PLA 3D Printer Filament 1KG"] = ("PLA", "Matte"),
        ["ASA 3D Printer Filament 1KG"] = ("ASA", null),
        ["E ABS(Easy ABS) 3D Printer Filament 1KG(p.s.: For New Refill Spool is 0.9kg)"] = ("ABS", "Easy"),
        ["Glow in The Dark (Luminous) PLA 3D Printer Filament 1KG"] = ("PLA", "Glow in the Dark"),
        ["TPU-SILK(SILK-Textured TPU) 3D Printer Filament 1KG"] = ("TPU", "Silk"),
        ["Twinkling 3D Printer PLA Filament 1KG"] = ("PLA", "Twinkling"),
        ["High Speed PLA(HS_PLA) 3D Printer Filament 1KG"] = ("PLA", "High Speed"),
        ["TPU 3D Printer Filament 1KG, TPU 90A/TPU 95A"] = ("TPU", null),
        ["PLA+ 2.0, Upgraded PLA+(PLA Plus), 3D Printer Filament 1KG"] = ("PLA", "Plus 2.0"),
        ["APLA (Anti-string PLA) 3D Printer Filament 1KG"] = ("PLA", "Anti-string"),
        ["PETG Rainbow Filament 3D Printer Filament 1KG"] = ("PETG", "Rainbow"),
        ["Optimized Wood PLA 3D Printer Filament 1KG, Optimized and Upgraded Wood Texture"] = ("PLA", "Wood"),
        ["PETG Glow in The Dark (Luminous) 3D Printer Filament 1KG"] = ("PETG", "Glow in the Dark"),
        ["High Speed Matte PETG 3D Printer Filament 1KG"] = ("PETG", "High Speed Matte"),
        ["PETG-CF(PETG Carbon Fiber) 3D Printer Filament 1KG"] = ("PETG-CF", null),
        ["High Speed Matte PLA 3D Printer Filament 1KG"] = ("PLA", "High Speed Matte"),
        ["High Speed PLA+(PLA Plus), HS_PLA+ 3D Printer Filament 1KG"] = ("PLA", "High Speed Plus"),
        ["High Speed PLA+ 2.0(HSPLA Plus 2.0), High Speed 3D Printer Filament 1KG"] = ("PLA", "High Speed Plus 2.0"),
        ["SUNLU PLA Galaxy 1KG, Color-Shifting PLA Esthenic Filament, Sparkling Ultrafine Pearlescent Powder"] = ("PLA", "Galaxy"),
        ["SUNLU Matte PLA Dual-Color 3D Printer Esthenic Filament 1KG, Seamless Two-Tone Shifts & Soft Matte Finish"] = ("PLA", "Matte Dual-Color"),
    };

    public static (string Material, string? Variant) SplitMaterialVariant(string title) =>
        MaterialVariantByTitle.TryGetValue(title, out var mv) ? mv : (title, null);

    private record ShopifyCollection(List<ShopifyProduct> Products);

    private record ShopifyProduct(string Title, List<ShopifyOption>? Options);

    private record ShopifyOption(string Name, List<string> Values);
}
