using System.Globalization;
using System.Text.RegularExpressions;

namespace Spoolbook.FilamentSync;

public static partial class HatchboxStoreParser
{
    // Products come from products.json via CloakBrowser (see CloakBrowserClient) — Chromium
    // wraps raw JSON responses in a <pre> tag when navigated to directly, and HTML-escapes it.
    [GeneratedRegex(@"<pre>(.*)</pre>", RegexOptions.Singleline)]
    private static partial Regex PreTagRegex();

    public static IReadOnlyList<string> ParseCollection(string htmlWrappedJson)
    {
        var match = PreTagRegex().Match(htmlWrappedJson);
        if (!match.Success) return [];

        var json = System.Net.WebUtility.HtmlDecode(match.Groups[1].Value);
        var data = System.Text.Json.JsonSerializer.Deserialize<ShopifyCollection>(json,
            new System.Text.Json.JsonSerializerOptions { PropertyNameCaseInsensitive = true });
        return (data?.Products ?? []).Select(p => p.Title.Trim()).ToList();
    }

    private record ShopifyCollection(List<ShopifyProduct> Products);
    private record ShopifyProduct(string Title);

    // Resin, gift cards, and merch don't contain "FILAMENT" in their title, so that check alone
    // excludes them; 3D-pen filament and sample packs are a different product category (not
    // spools); "Exclusive Release" is a one-off whose title carries no parseable color/material
    // (its actual color-change behavior is only described in body_html).
    private static readonly string[] ExcludePatterns = ["3D PEN", "SAMPLE PACK", "CLEANING"];
    private const string ExcludeExact = "Exclusive Release - Temp Change Filament";

    public static IReadOnlyList<string> FilterRealProducts(IEnumerable<string> titles) =>
        titles.Where(t => t.Contains("FILAMENT", StringComparison.OrdinalIgnoreCase)
            && t != ExcludeExact
            && !ExcludePatterns.Any(p => t.Contains(p, StringComparison.OrdinalIgnoreCase)))
            .ToList();

    private static readonly string[] Materials = ["PLA", "ABS", "PETG", "TPU", "PA", "PC"];

    // Product-line descriptors that sit next to color/material rather than being a color
    // themselves — stripped from both ends of the remaining text (iteratively, since some
    // products stack more than one, e.g. "Stone Gray Matte"), same algorithm as
    // ProtopastaStoreParser (arbitrary, artisan-style naming needs the same iterative approach).
    private static readonly string[] Qualifiers =
    [
        "Metallic Finish", "Temperature Color Changing", "UV Color Changing",
        "Glow In The Dark", "Carbon Fiber", "Paint Free", "Performance",
        "Transparent", "Silk", "Stone", "Wood", "Sparkle", "Matte", "Rapid",
        "Max V2", "Pro+",
    ];

    private static (string Text, List<string> Matched) StripQualifiers(string text)
    {
        var matched = new List<string>();
        bool changed;
        do
        {
            changed = false;
            foreach (var q in Qualifiers)
            {
                var qu = q.ToUpperInvariant();
                if (text.Equals(qu, StringComparison.Ordinal))
                {
                    text = ""; matched.Add(q); changed = true; break;
                }
                if (text.EndsWith(" " + qu, StringComparison.Ordinal))
                {
                    text = text[..^(qu.Length + 1)]; matched.Add(q); changed = true; break;
                }
                if (text.StartsWith(qu + " ", StringComparison.Ordinal))
                {
                    text = text[(qu.Length + 1)..]; matched.Add(q); changed = true; break;
                }
            }
        } while (changed);

        return (text.Trim(), matched);
    }

    private static string ToTitleCase(string s) =>
        CultureInfo.InvariantCulture.TextInfo.ToTitleCase(s.ToLowerInvariant());

    // Diameter/weight/packaging notes always come after "FILAMENT" in the title (e.g. "- 1.75MM,
    // 1KG SPOOL", "REFILL & RELOADABLE SPOOL", "(SHORE 95A)") — irrelevant to Material/Variant/
    // Color, and stripping at "FILAMENT" sidesteps needing to handle the inconsistent hyphen vs.
    // en-dash the store uses before the size info.
    public static (string Material, string? Variant, string Color) ParseProductTitle(string rawTitle)
    {
        var beforeFilament = Regex.Split(rawTitle.Trim(), "FILAMENT", RegexOptions.IgnoreCase)[0].Trim();

        foreach (var material in Materials)
        {
            var m = Regex.Match(beforeFilament, $@"\b{material}\b");
            if (!m.Success) continue;

            var remainder = Regex.Replace(
                (beforeFilament[..m.Index] + " " + beforeFilament[(m.Index + m.Length)..]).Trim(),
                @"\s+", " ");
            var (colorText, matchedQualifiers) = StripQualifiers(remainder);

            matchedQualifiers.Reverse();
            var variant = matchedQualifiers.Count > 0 ? string.Join(" ", matchedQualifiers) : null;
            var color = colorText.Length == 0 ? "Natural" : ToTitleCase(colorText);
            var materialName = material == "PA" ? "Nylon" : material;
            return (materialName, variant, color);
        }

        return (beforeFilament, null, beforeFilament);
    }
}
