using System.Text.RegularExpressions;

namespace Spoolbook.FilamentSync;

// colorFabb is Magento-based, not Shopify — each color is its own standalone product page
// (confirmed via a sample page's "alternative color" widget, which turned out to be a
// cross-sell link to sibling materials, not a variant selector), and the listing page's own
// link text already carries the full product title, so no per-product page fetch is needed.
public static partial class ColorfabbStoreParser
{
    [GeneratedRegex("""<a[^>]*class="product-item-link"[^>]*href="([^"]+)"[^>]*>([^<]*)</a>""")]
    private static partial Regex ProductLinkRegex();

    public static IReadOnlyList<string> ExtractProductTitles(string listingHtml) =>
        ProductLinkRegex().Matches(listingHtml).Select(m => m.Groups[2].Value.Trim()).ToList();

    // "Luvocom"/"IGUS" are third-party industrial materials resold (not colorFabb-branded)
    // "VALUE PACK" is a multi-color bundle, "PLAQUE SAMPLE" duplicates a real color as a swatch.
    private static readonly string[] ExcludePatterns = ["Luvocom", "IGUS", "VALUE PACK", "PLAQUE SAMPLE"];

    public static IReadOnlyList<string> FilterRealProducts(IEnumerable<string> titles) =>
        titles.Where(t => !ExcludePatterns.Any(t.Contains)).ToList();

    // The "XXXfill" composite-material lines (metal/mineral/wood-filled PLA) are single-color —
    // the fill name itself is the closest thing to a color description.
    private static readonly Dictionary<string, string> FillVariants = new(StringComparer.OrdinalIgnoreCase)
    {
        ["STEELFILL"] = "steelFill", ["CORKFILL"] = "corkFill", ["COPPERFILL"] = "copperFill",
        ["BRONZEFILL"] = "bronzeFill", ["WOODFILL"] = "woodFill", ["GLOWFILL"] = "glowFill",
    };
    private static readonly Dictionary<string, string> FillColors = new(StringComparer.OrdinalIgnoreCase)
    {
        ["steelFill"] = "Steel", ["corkFill"] = "Cork", ["copperFill"] = "Copper",
        ["bronzeFill"] = "Bronze", ["woodFill"] = "Wood", ["glowFill"] = "Glow",
    };

    // "Smokey Black"/"Milky White" are curated RAL-line favorites without an explicit RAL code
    // in the title (confirmed via breadcrumb: PLA Filaments > RAL Favorites).
    private static readonly string[] RalFavorites = ["Smokey Black", "Milky White"];

    // A few product codes carry no separable color word at all (confirmed via product pages) —
    // "Natural" is the closest honest description available, not a guess.
    private static readonly Dictionary<string, string> CodeOnlyMaterials = new(StringComparer.OrdinalIgnoreCase)
    {
        ["nGen-CF10"] = "nGen-CF10", ["XT-CF20"] = "XT-CF20", ["PA Neat"] = "PA", ["PA-CF low warp"] = "PA-CF",
    };

    // Longest/most-specific first so e.g. "LW-PLA-HT" matches before "LW-PLA", "rPLA" before "PLA".
    private static readonly string[] KnownMaterials =
    [
        "LW-PLA-HT", "LW-PLA", "LW-ASA", "PLA-HP", "PLA/PHA", "allPHA", "rPETG", "rPLA",
        "PETG", "PLA", "ASA", "PA-CF", "PA", "TPU 95A", "TPU 85A", "TPU",
        "XT-CF20", "XT", "nGen-CF10", "nGen Flex", "nGen"
    ];

    // Product-line qualifiers that appear alongside the material but aren't part of it — kept
    // as Variant so e.g. "PLA Economy Black" and "PLA Black" aren't conflated.
    private static readonly string[] VariantQualifiers =
        ["High Speed Pro", "Semi-Matte", "Semi Matte", "Economy", "Regrind",
         "Chameleon", "Vertigo", "Vibers", "Prosthetic", "Metal Detectable", "Varioshore"];

    [GeneratedRegex(@"\s*\d+(\.\d+)?\s*/\s*\d+\s*$")]
    private static partial Regex TrailingSizeSpecRegex();

    private static string TitleCase(string s) =>
        string.Join(' ', s.Split(' ', StringSplitOptions.RemoveEmptyEntries)
            .Select(w => char.ToUpperInvariant(w[0]) + w[1..].ToLowerInvariant()));

    public static (string Material, string? Variant, string Color) ParseProductTitle(string rawTitle)
    {
        var t = rawTitle.Trim();

        if (Regex.IsMatch(t, @"RAL \d+") || RalFavorites.Contains(t))
            return ("PLA", "RAL", t);

        if (FillVariants.TryGetValue(t, out var fillVariant))
            return ("PLA", fillVariant, FillColors[fillVariant]);

        if (t.StartsWith("STONEFILL", StringComparison.OrdinalIgnoreCase))
            return ("PLA", "stoneFill", TitleCase(t["STONEFILL".Length..].Trim()));

        if (t.StartsWith("VARIOSHORE PROSTHETIC", StringComparison.OrdinalIgnoreCase))
            return ("TPU", "Varioshore Prosthetic", TitleCase(t["VARIOSHORE PROSTHETIC".Length..].Trim()));

        if (CodeOnlyMaterials.TryGetValue(t, out var codeMaterial))
            return (codeMaterial, null, "Natural");

        t = t.Replace('_', ' ');
        t = TrailingSizeSpecRegex().Replace(t, "");
        t = Regex.Replace(t, @"\s+", " ").Trim();

        var material = KnownMaterials.FirstOrDefault(m => Regex.IsMatch(t, $@"\b{Regex.Escape(m)}\b", RegexOptions.IgnoreCase));
        if (material is null) return (t, null, t);

        var rest = Regex.Replace(t, $@"\b{Regex.Escape(material)}\b", "", RegexOptions.IgnoreCase).Trim(' ', '-');

        var variantQualifier = VariantQualifiers.FirstOrDefault(v => Regex.IsMatch(rest, $@"\b{Regex.Escape(v)}\b", RegexOptions.IgnoreCase));
        string? variant = null;
        if (variantQualifier is not null)
        {
            rest = Regex.Replace(rest, $@"\b{Regex.Escape(variantQualifier)}\b", "", RegexOptions.IgnoreCase).Trim(' ', '-');
            variant = TitleCase(variantQualifier);
        }

        rest = Regex.Replace(rest, @"[\s-]+", " ").Trim(' ', '-');
        return (material, variant, rest.Length == 0 ? "Natural" : rest);
    }
}
