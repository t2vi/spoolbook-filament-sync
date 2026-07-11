using System.Text.RegularExpressions;

namespace Spoolbook.FilamentSync;

// Jaycar's own house brand (no independent manufacturer storefront exists — see
// docs/adr/0012's "reversing abandon if blocked" addendum) — jaycar.com.au is Slic3D's own
// store in the same sense hatchbox3d.com is Hatchbox's. Blocked by DataDome (see
// CloakBrowserClient), so fetched the same way. Small brand (17 SKUs, single page, no
// pagination) with no products.json-style API — the Next.js page ships product data purely
// client-side, but each product image's alt text is a clean, human-readable title
// ("Slic3D PETG filament Black 1.75mm 1kg"), so no need to reverse-engineer the API.
public static partial class Slic3DStoreParser
{
    [GeneratedRegex("alt=\"(Slic3D[^\"]*)\"")]
    private static partial Regex AltTextRegex();

    public static IReadOnlyList<string> ParseListingPage(string html) =>
        AltTextRegex().Matches(html).Select(m => m.Groups[1].Value).Distinct().ToList();

    [GeneratedRegex(@"\bfilament\b", RegexOptions.IgnoreCase)]
    private static partial Regex FilamentWordRegex();

    // Handles both "1.75mm 1kg" and the real "Yellow1.75mm 1kg" (no space before size).
    [GeneratedRegex(@"\d+(\.\d+)?\s*mm\s*\d+\s*kg", RegexOptions.IgnoreCase)]
    private static partial Regex SizeRegex();

    [GeneratedRegex(@"\bSpool-Less\b", RegexOptions.IgnoreCase)]
    private static partial Regex SpoolLessRegex();

    private static readonly string[] Materials = ["PETG", "PLA"];

    public static (string Material, string? Variant, string Color) ParseProductTitle(string altText)
    {
        var t = altText.Replace("Slic3D", "", StringComparison.OrdinalIgnoreCase).Trim();
        t = FilamentWordRegex().Replace(t, "");
        t = SizeRegex().Replace(t, "");

        string? variant = null;
        if (SpoolLessRegex().IsMatch(t))
        {
            variant = "Spool-Less";
            t = SpoolLessRegex().Replace(t, "");
        }

        var material = "Unknown";
        foreach (var m in Materials)
        {
            if (!Regex.IsMatch(t, $@"\b{m}\b")) continue;
            material = m;
            t = Regex.Replace(t, $@"\b{m}\b", "");
            break;
        }

        var color = Regex.Replace(t, @"\s+", " ").Trim();
        return (material, variant, color);
    }
}
