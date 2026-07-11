namespace Spoolbook.FilamentSync.Tests;

public class ElegooStoreParserTests
{
    [Fact]
    public void ParseCollection_ExtractsTitleAndColors_SkipsBulkAndNonFilamentProducts()
    {
        var json = """
            {
              "products": [
                {
                  "title": "PLA Plus",
                  "product_type": "3D Filaments",
                  "options": [{ "name": "Color", "position": 1, "values": ["Black", "White"] }]
                },
                {
                  "title": "PLA Filament 1.75mm Black 10KG",
                  "product_type": "3D Filaments",
                  "options": [{ "name": "Color", "position": 1, "values": ["Black"] }]
                },
                {
                  "title": "Mini 250 g Filament Bundle",
                  "product_type": "3D Filaments",
                  "options": [{ "name": "Color", "position": 1, "values": ["Mixed"] }]
                },
                {
                  "title": "3D Stainless Steel Funnel",
                  "product_type": "Accessories",
                  "options": [{ "name": "Title", "position": 1, "values": ["Default Title"] }]
                }
              ]
            }
            """;

        var products = ElegooStoreParser.ParseCollection(json);

        var product = Assert.Single(products);
        Assert.Equal("PLA Plus", product.Title);
        Assert.Equal(["Black", "White"], product.Colors);
    }

    [Fact]
    public void ParseCollection_FiltersBulkPackPseudoColorsWithinAProduct()
    {
        // Some products (e.g. "PLA Matte") mix real colors with bulk-pack pseudo-options
        // directly inside the same Color list, rather than as a separate bulk product.
        var json = """
            {
              "products": [
                {
                  "title": "PLA Matte",
                  "product_type": "3D Filaments",
                  "options": [{ "name": "Color", "position": 1, "values": ["Black", "2kg option1", "4KG-OPTION1"] }]
                }
              ]
            }
            """;

        var products = ElegooStoreParser.ParseCollection(json);

        var product = Assert.Single(products);
        Assert.Equal(["Black"], product.Colors);
    }

    [Theory]
    [InlineData("PLA Plus", "PLA", "Plus")]
    [InlineData("Rapid PLA Plus", "PLA", "Rapid Plus")]
    [InlineData("PLA Silk", "PLA", "Silk")]
    [InlineData("ASA", "ASA", null)]
    [InlineData("Rapid PETG", "PETG", "Rapid")]
    [InlineData("TPU 95A", "TPU", "95A")]
    [InlineData("PLA-CF", "PLA-CF", null)]
    [InlineData("PC", "PC", null)]
    public void SplitMaterialVariant_MatchesKnownMaterialToken(string title, string expectedMaterial, string? expectedVariant)
    {
        var (material, variant) = ElegooStoreParser.SplitMaterialVariant(title);

        Assert.Equal(expectedMaterial, material);
        Assert.Equal(expectedVariant, variant);
    }
}
