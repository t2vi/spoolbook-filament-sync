using System.Text.Json;
using System.Text.RegularExpressions;

namespace Spoolbook.FilamentSync;

public static partial class PrusamentStoreParser
{
    private static readonly JsonSerializerOptions JsonOptions = new() { PropertyNameCaseInsensitive = true };

    // Unlike the Shopify-backed stores (Elegoo/SUNLU/Polymaker), each Prusament "product" node
    // here already IS a single color — no separate per-product Color option list.
    public static IReadOnlyList<string> ParseCollection(string json)
    {
        var data = JsonSerializer.Deserialize<GraphQlResponse>(json, JsonOptions);
        var edges = data?.Data?.Category?.Products?.Edges ?? [];

        return edges
            .Select(e => e.Node.Name)
            .Where(n => n is not null)
            .Cast<string>()
            // Samples (25g) and multipacks resell the same color as the regular-size listing.
            .Where(n => !n.Contains("sample", StringComparison.OrdinalIgnoreCase)
                     && !n.Contains("multipack", StringComparison.OrdinalIgnoreCase))
            .ToList();
    }

    // Longest/most-specific first: "PEI 1010" and "TPU 95A" are atomic grade names, not a
    // material + a separate color-prefix number.
    private static readonly string[] KnownMaterials =
        ["TPU 95A", "PEI 1010", "PA11", "PETG", "PLA", "ASA", "PC", "PEI", "PP", "PVB", "TPU"];

    private static readonly string[] VariantQualifiers = ["Carbon Fiber", "Glass Fiber", "Blend"];

    [GeneratedRegex(@"\s*\(?\d+(\.\d+)?\s*(g|kg)\)?.*$", RegexOptions.IgnoreCase)]
    private static partial Regex WeightSuffixRegex();

    public static (string Material, string? Variant, string Color) ParseProductName(string rawName)
    {
        var t = rawName;
        if (t.StartsWith("Prusament ")) t = t["Prusament ".Length..];

        var premium = t.StartsWith("Premium ");
        if (premium) t = t["Premium ".Length..];

        t = WeightSuffixRegex().Replace(t, "").Trim();

        var material = KnownMaterials.FirstOrDefault(m => t.StartsWith(m + " ", StringComparison.Ordinal) || t == m);
        if (material is null) return (t, null, "");

        var rest = t[material.Length..].Trim();

        var variantQualifier = VariantQualifiers.FirstOrDefault(v => rest.StartsWith(v + " ", StringComparison.Ordinal) || rest == v);
        if (variantQualifier is not null) rest = rest[variantQualifier.Length..].Trim();

        var variantParts = new List<string>();
        if (premium) variantParts.Add("Premium");
        if (variantQualifier is not null) variantParts.Add(variantQualifier);
        var variant = variantParts.Count == 0 ? null : string.Join(' ', variantParts);

        return (material, variant, rest);
    }

    private record GraphQlResponse(GraphQlData? Data);
    private record GraphQlData(CategoryData? Category);
    private record CategoryData(ProductsData? Products);
    private record ProductsData(List<ProductEdge> Edges);
    private record ProductEdge(ProductNode Node);
    private record ProductNode(string? Name);
}
