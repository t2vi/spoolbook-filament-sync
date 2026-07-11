using System.Text.RegularExpressions;

namespace Spoolbook.FilamentSync;

public record ProductPage(string? Name, IReadOnlyList<string> Colors);

public static partial class BambuStoreParser
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
        var nameMatch = H1Regex().Match(html);
        var name = nameMatch.Success ? nameMatch.Groups[1].Value.Trim() : null;

        var colors = new List<string>();
        foreach (Match m in ColorSwatchRegex().Matches(html))
        {
            var color = m.Groups[1].Value.Trim();
            if (!colors.Contains(color)) colors.Add(color);
        }

        return new ProductPage(name, colors);
    }

    public static string StripMaterialPrefix(string color, string material) =>
        color.StartsWith(material + " ", StringComparison.Ordinal) ? color[(material.Length + 1)..] : color;

    public static (string Material, string? Variant) SplitMaterialVariant(string productName)
    {
        var spaceIndex = productName.IndexOf(' ');
        return spaceIndex < 0
            ? (productName, null)
            : (productName[..spaceIndex], productName[(spaceIndex + 1)..]);
    }

    [GeneratedRegex("href=\"/en/products/([a-z0-9-]+)\"")]
    private static partial Regex ProductLinkRegex();

    [GeneratedRegex("<h1[^>]*>(.*?)</h1>", RegexOptions.Singleline)]
    private static partial Regex H1Regex();

    [GeneratedRegex("<li value=\"([^\"(]+) \\(\\d+\\)\"")]
    private static partial Regex ColorSwatchRegex();
}
