using System.Text.RegularExpressions;

namespace Spoolbook.FilamentSync;

public static partial class EsunStoreParser
{
    public static IReadOnlyList<string> ExtractProductSlugs(string listingHtml)
    {
        var seen = new List<string>();
        foreach (Match m in ProductLinkRegex().Matches(listingHtml))
        {
            var slug = m.Groups[1].Value;
            if (!seen.Contains(slug)) seen.Add(slug);
        }
        return seen;
    }

    public static ProductPage ParseProductPage(string html)
    {
        var nameMatch = FilamentTitleRegex().Match(html);
        var name = nameMatch.Success ? nameMatch.Groups[1].Value.Trim() : null;

        // Scope color extraction to the Color <li> block specifically — the same
        // data-title/data-is-picture markup is reused elsewhere on the page for bundle/addon
        // pickers (e.g. multi-roll packs, "eSpool+" upsell), which aren't real colors.
        var colorBlockMatch = ColorBlockRegex().Match(html);
        var colorBlock = colorBlockMatch.Success ? colorBlockMatch.Groups[1].Value : "";

        var colors = new List<string>();
        foreach (Match m in ColorSwatchRegex().Matches(colorBlock))
        {
            var color = m.Groups[1].Value.Trim();
            if (!colors.Contains(color)) colors.Add(color);
        }

        return new ProductPage(name, colors);
    }

    public static (string Material, string? Variant) SplitMaterialVariant(string name)
    {
        var plusIndex = name.IndexOf('+');
        if (plusIndex >= 0)
        {
            var material = name[..plusIndex];
            var rest = name[(plusIndex + 1)..].Trim();
            return (material, rest.Length == 0 ? "Plus" : $"Plus {rest}");
        }

        var dashIndex = name.IndexOf('-');
        if (dashIndex >= 0)
            return (name[..dashIndex], name[(dashIndex + 1)..]);

        return (name, null);
    }

    [GeneratedRegex("href=\"/products/([a-z0-9-]+)\"")]
    private static partial Regex ProductLinkRegex();

    [GeneratedRegex("""<h1[^>]*>eSUN\s+(.+?)\s+[\d.]+mm 3D Filament.*?</h1>""", RegexOptions.Singleline)]
    private static partial Regex FilamentTitleRegex();

    // The wrapping <li>'s data-type is "color" for hex swatches or "picture" for photo
    // thumbnails, depending on the product — name="Color" is the constant across both.
    [GeneratedRegex("""name="Color"[^>]*data-type="(?:color|picture)".*?>(.*?)</li>""", RegexOptions.Singleline)]
    private static partial Regex ColorBlockRegex();

    // Color options render either as a plain hex swatch (data-is-picture="0") or a product
    // photo thumbnail (data-is-picture="1") — both are real colors, only the former carries hex.
    [GeneratedRegex("data-title=\"([^\"]+)\"\\s+data-is-picture=\"[01]\"")]
    private static partial Regex ColorSwatchRegex();
}
