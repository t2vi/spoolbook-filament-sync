namespace Spoolbook.FilamentSync.Tests;

public class PolymakerStoreParserTests
{
    [Fact]
    public void ParseCollection_KeepsOnlyFilamentProductTypes()
    {
        var json = """
            {
              "products": [
                {
                  "title": "PolyLite™ PLA",
                  "product_type": "Polymaker Filament",
                  "options": [{ "name": "Color", "values": ["Black", "White"] }]
                },
                {
                  "title": "Blue Starter Pack",
                  "product_type": "Bundle Packs",
                  "options": [{ "name": "Color", "values": ["Mixed"] }]
                },
                {
                  "title": "Creator Special Edition: Hedgehog",
                  "product_type": "Creator Spools",
                  "options": [{ "name": "Color", "values": ["Mixed"] }]
                }
              ]
            }
            """;

        var products = PolymakerStoreParser.ParseCollection(json);

        var product = Assert.Single(products);
        Assert.Equal("PolyLite™ PLA", product.Title);
    }

    [Fact]
    public void ParseCollection_SkipsProductsWhereColorOptionIsNotActuallyColors()
    {
        // "PolyLite CosPLA"'s "Color" option is really a formula-variant selector
        // ("Version A - Durability...", "Version B - Sand-ability...."), not real colors.
        var json = """
            {
              "products": [
                {
                  "title": "PolyLite™ CosPLA",
                  "product_type": "Polymaker Filament",
                  "options": [{ "name": "Color", "values": ["Version A - Durability with extra sand-ability", "Version B - Sand-ability with extra durability"] }]
                }
              ]
            }
            """;

        var products = PolymakerStoreParser.ParseCollection(json);

        Assert.Empty(products);
    }

    [Fact]
    public void ParseCollection_SkipsRefillProductsWithUnattributableColors()
    {
        // "Panchroma PLA Refill" mixes colors from Matte/Silk/Gradient/Marble sub-lines into
        // one flat 110-color list with no way to tell which sub-line each color belongs to.
        var json = """
            {
              "products": [
                {
                  "title": "Panchroma™ PLA Refill",
                  "product_type": "Panchroma Filament",
                  "options": [{ "name": "Color", "values": ["Matte Black", "Silk Gold"] }]
                }
              ]
            }
            """;

        var products = PolymakerStoreParser.ParseCollection(json);

        Assert.Empty(products);
    }

    [Theory]
    [InlineData("PolyLite™ PLA", "PLA", "PolyLite")]
    [InlineData("PolyLite™ PLA Pro", "PLA", "PolyLite Pro")]
    [InlineData("Panchroma™ Matte PLA", "PLA", "Panchroma Matte")]
    [InlineData("Panchroma™ CoPE", "CoPE", "Panchroma")]
    [InlineData("PolyMax™ PC-FR", "PC-FR", "PolyMax")]
    [InlineData("Polymaker PC-ABS", "PC-ABS", "Polymaker")]
    [InlineData("Polymaker™ HT-PLA-GF", "HT-PLA-GF", "Polymaker")]
    [InlineData("PolyLite™ LW-PLA", "LW-PLA", "PolyLite")]
    [InlineData("Fiberon™ ASA-CF08", "ASA-CF08", null)]
    [InlineData("Fiberon™ PETG-ESD", "PETG-ESD", null)]
    [InlineData("PolyCast™", "PLA", "PolyCast")]
    [InlineData("PolySmooth™", "PVB", "PolySmooth")]
    [InlineData("PolyDissolve™ S1 (PVA)", "PVA", "PolyDissolve S1")]
    [InlineData("PolySupport™ for PA12", "PA12", "PolySupport")]
    [InlineData("Panchroma™ Gradient Galaxy", "PLA", "Panchroma Gradient Galaxy")]
    public void SplitMaterialVariant_HandlesBrandedTitles(string title, string expectedMaterial, string? expectedVariant)
    {
        var (material, variant) = PolymakerStoreParser.SplitMaterialVariant(title);

        Assert.Equal(expectedMaterial, material);
        Assert.Equal(expectedVariant, variant);
    }
}
