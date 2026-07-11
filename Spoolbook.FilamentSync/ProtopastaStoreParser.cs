using System.Text.Json;
using System.Text.Json.Serialization;
using System.Text.RegularExpressions;

namespace Spoolbook.FilamentSync;

public static partial class ProtopastaStoreParser
{
    private static readonly JsonSerializerOptions JsonOptions = new() { PropertyNameCaseInsensitive = true };

    // Every color is its own standalone product (no per-product color options), so the listing
    // title alone is enough — no per-product page fetch needed.
    public static IReadOnlyList<string> ParseCollection(string json)
    {
        var data = JsonSerializer.Deserialize<ShopifyCollection>(json, JsonOptions);
        return (data?.Products ?? [])
            .Where(p => p.ProductType == "3D Printer Filament")
            .Select(p => p.Title)
            .ToList();
    }

    private record ShopifyCollection(List<ShopifyProduct> Products);

    private record ShopifyProduct(string Title, [property: JsonPropertyName("product_type")] string ProductType);

    // Subscription boxes aren't single-color SKUs; the charity/collab one-off and the two
    // "Glow-in-the-Dark" exclusives state no material anywhere (title or description) so
    // there's nothing honest to classify them as.
    private static readonly string[] ExcludePatterns = ["Subscription", "Glow-in-the-Dark"];
    private const string ExcludeExact = "\"We Keep Us Safe\" by The Whistle Crew";

    public static IReadOnlyList<string> FilterRealProducts(IEnumerable<string> titles) =>
        titles.Where(t => t != ExcludeExact && !ExcludePatterns.Any(t.Contains)).ToList();

    // Longest/most-specific first so "HTPLA"/"HFPLA" match before the bare "PLA" they contain.
    private static readonly string[] KnownMaterials = ["HTPLA", "HFPLA", "PCTG", "PETG", "TPU", "TPE", "Polyketone", "PLA"];

    // Product-line descriptors that sit next to the material/color rather than being a color
    // themselves — stripped from both ends of the remaining text (iteratively, since some
    // products stack more than one, e.g. "Static Dissipative Carbon Fiber"), longest first.
    private static readonly string[] VariantQualifiers =
    [
        "High Strength Carbon Fiber", "High Impact Carbon Fiber", "Carbon Fiber Composite",
        "Recycled Carbon Fiber", "Metal Composite", "Glass Fiber", "Carbon Fiber",
        "Static Dissipative", "Quantum Dot", "Ice Translucent", "Electrically Conductive",
        "High Density", "High Flow", "Matte Fiber", "Glitter Flake",
        "Multicolor", "Translucent", "Metallic", "Glitter", "Opaque", "Reflective",
        "Thermochromic", "c-Matte", "Marble", "Rigid", "Flexible", "Premium Basic",
        "Recycled", "Composite", "Smooth",
    ];

    [GeneratedRegex(@"\s*\*[^*]+\*\s*$")]
    private static partial Regex TrailingPromoRegex();

    [GeneratedRegex(@"\s*\(HFPLA\)")]
    private static partial Regex HfplaParenRegex();

    [GeneratedRegex(@"^(.+?)\s+PLA\s*-\s*Premium Basic PLA$")]
    private static partial Regex PremiumBasicRegex();

    [GeneratedRegex(@"^Matte Fiber (HTPLA|PLA)\s*-\s*(.+)$")]
    private static partial Regex MatteFiberRegex();

    [GeneratedRegex(@"^Stainless Steel Metal Composite PLA\s*-\s*(.+?)\s*color$")]
    private static partial Regex StainlessSteelRegex();

    // "{Color} [Material] with {Accent} Glitter" — the accent is part of the color's own name,
    // not a separate product line, and these specialty one-offs are always Protopasta's HTPLA
    // (confirmed via product descriptions) even on the few titles that omit the material word.
    [GeneratedRegex(@"^(.+?)(?:\s+(HTPLA|PLA))?\s+with\s+(.+)$")]
    private static partial Regex WithAccentRegex();

    [GeneratedRegex(@"^(.*?)-filled$")]
    private static partial Regex FilledSuffixRegex();

    private static (string Text, List<string> Matched) StripQualifiers(string text)
    {
        var matched = new List<string>();
        bool changed;
        do
        {
            changed = false;
            foreach (var q in VariantQualifiers)
            {
                if (text.Equals(q, StringComparison.Ordinal))
                {
                    text = ""; matched.Add(q); changed = true; break;
                }
                if (text.EndsWith(" " + q, StringComparison.Ordinal))
                {
                    text = text[..^(q.Length + 1)]; matched.Add(q); changed = true; break;
                }
                if (text.StartsWith(q + " ", StringComparison.Ordinal))
                {
                    text = text[(q.Length + 1)..]; matched.Add(q); changed = true; break;
                }
            }
        } while (changed);

        return (text.Trim(), matched);
    }

    public static (string Material, string? Variant, string Color) ParseProductTitle(string rawTitle)
    {
        var t = TrailingPromoRegex().Replace(rawTitle.Trim(), "");
        t = HfplaParenRegex().Replace(t, "").Trim();

        var premiumBasic = PremiumBasicRegex().Match(t);
        if (premiumBasic.Success)
            return ("PLA", "Premium Basic", premiumBasic.Groups[1].Value.Trim());

        var matteFiber = MatteFiberRegex().Match(t);
        if (matteFiber.Success)
            return (matteFiber.Groups[1].Value, "Matte Fiber", matteFiber.Groups[2].Value.Trim());

        var stainlessSteel = StainlessSteelRegex().Match(t);
        if (stainlessSteel.Success)
            return ("PLA", "Stainless Steel Metal Composite", stainlessSteel.Groups[1].Value.Trim());

        var withAccent = WithAccentRegex().Match(t);
        if (withAccent.Success)
        {
            var material = withAccent.Groups[2].Success ? withAccent.Groups[2].Value : "HTPLA";
            return (material, null, $"{withAccent.Groups[1].Value.Trim()} with {withAccent.Groups[3].Value.Trim()}");
        }

        string? matchedMaterial = null;
        var remainder = t;
        foreach (var material in KnownMaterials)
        {
            var m = Regex.Match(t, $@"\b{Regex.Escape(material)}\b");
            if (!m.Success) continue;
            matchedMaterial = material;
            remainder = Regex.Replace((t[..m.Index] + " " + t[(m.Index + m.Length)..]).Trim(), @"\s+", " ");
            break;
        }

        if (matchedMaterial is null) return (t, null, t);

        var (colorText, matchedQualifiers) = StripQualifiers(remainder);
        var filledMatch = FilledSuffixRegex().Match(colorText);
        if (filledMatch.Success) colorText = filledMatch.Groups[1].Value.Trim();

        matchedQualifiers.Reverse();
        string? variant = matchedQualifiers.Count > 0 ? string.Join(" ", matchedQualifiers) : null;
        return (matchedMaterial, variant, colorText.Length == 0 ? "Natural" : colorText);
    }
}
