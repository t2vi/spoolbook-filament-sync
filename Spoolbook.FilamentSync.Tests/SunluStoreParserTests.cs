namespace Spoolbook.FilamentSync.Tests;

public class SunluStoreParserTests
{
    [Fact]
    public void ParseCollection_KeepsMoqListings_StripsMoqPrefixFromTitle()
    {
        // MOQ-tagged listings on SUNLU's store aren't duplicates of a retail SKU — for some
        // materials (SILK, ASA, TPU) they're the only listing that exists.
        var json = """
            {
              "products": [
                {
                  "title": "[MOQ: 6KG] SILK 3D Printer Filament 1KG",
                  "options": [{ "name": "Color", "values": ["Black", "White"] }]
                }
              ]
            }
            """;

        var products = SunluStoreParser.ParseCollection(json);

        var product = Assert.Single(products);
        Assert.Equal("SILK 3D Printer Filament 1KG", product.Title);
    }

    [Fact]
    public void ParseCollection_SkipsBulkSizeRepackagings()
    {
        var json = """
            {
              "products": [
                {
                  "title": "PLA Large Spool 3D Printer Filament 5KG",
                  "options": [{ "name": "Color", "values": ["Black"] }]
                },
                {
                  "title": "PLA 3KG Large Spool 3D Printer Filament 3KG",
                  "options": [{ "name": "Color", "values": ["Black"] }]
                },
                {
                  "title": "ABS 3D Printer Filament 0.9KG/1KG",
                  "options": [{ "name": "Color", "values": ["Black"] }]
                }
              ]
            }
            """;

        var products = SunluStoreParser.ParseCollection(json);

        // Only the real 1kg single-roll product survives — "0.9KG/1KG" is normal single-roll
        // fill-weight labeling, not a bulk repackaging, so it must not false-positive.
        var product = Assert.Single(products);
        Assert.Equal("ABS 3D Printer Filament 0.9KG/1KG", product.Title);
    }

    [Fact]
    public void ParseCollection_SkipsMultiMaterialBundlesWithUnattributableColors()
    {
        // "Basic Filament Collection" mixes colors from 6+ materials into one flat Color list
        // with no way to tell which color belongs to which material.
        var json = """
            {
              "products": [
                {
                  "title": "[Australia Only] SUNLU Basic Filament Collection – PLA, PLA+, PETG",
                  "options": [{ "name": "Color", "values": ["Black", "White", "Grey"] }]
                },
                {
                  "title": "TPU-SILK(SILK-Textured TPU) 3D Printer Filament 1KG",
                  "options": [{ "name": "Color", "values": ["Black", "Cream White"] }]
                }
              ]
            }
            """;

        var products = SunluStoreParser.ParseCollection(json);

        // TPU-SILK is a real single product (Silk-textured TPU), not a 2-material bundle —
        // "SILK" isn't counted as a distinct material since SUNLU uses it inconsistently as
        // both a standalone material and a texture descriptor.
        var product = Assert.Single(products);
        Assert.Equal("TPU-SILK(SILK-Textured TPU) 3D Printer Filament 1KG", product.Title);
    }

    [Theory]
    [InlineData("SUNLU PLA+(PLA Plus) 3D Printer Filament 1KG", "PLA", "Plus")]
    [InlineData("SILK 3D Printer Filament 1KG", "PLA", "Silk")]
    [InlineData("ABS 3D Printer Filament 0.9KG/1KG", "ABS", null)]
    [InlineData("TPU-SILK(SILK-Textured TPU) 3D Printer Filament 1KG", "TPU", "Silk")]
    [InlineData("PETG-CF(PETG Carbon Fiber) 3D Printer Filament 1KG", "PETG-CF", null)]
    [InlineData("High Speed PLA+ 2.0(HSPLA Plus 2.0), High Speed 3D Printer Filament 1KG", "PLA", "High Speed Plus 2.0")]
    public void SplitMaterialVariant_LooksUpKnownTitle(string title, string expectedMaterial, string? expectedVariant)
    {
        var (material, variant) = SunluStoreParser.SplitMaterialVariant(title);

        Assert.Equal(expectedMaterial, material);
        Assert.Equal(expectedVariant, variant);
    }

    [Theory]
    [InlineData("1KG ABS | Black", "Black")]
    [InlineData("High Speed Matte PETG | Black", "Black")]
    [InlineData("PLA Galaxy | Starlit Flow", "Starlit Flow")]
    [InlineData("Anti-string PLA / Black", "Black")]
    [InlineData("PETG White", "White")]
    [InlineData("PLA+ Black", "Black")]
    [InlineData("PLA+ 2.0 | Black", "Black")]
    [InlineData("Matte White", "White")]
    [InlineData("Silk Black", "Black")]
    [InlineData("Twinkling Blue", "Blue")]
    [InlineData("Cherry Wood 1KG", "Cherry Wood")]
    [InlineData("Red Filament (Glow Red)", "Red Filament (Glow Red)")]
    [InlineData("Red+Yellow", "Red+Yellow")]
    public void ParseCollection_StripsRedundantMaterialPrefixFromColorNames(string rawColor, string expectedClean)
    {
        var json = $$"""
            {
              "products": [
                {
                  "title": "Test Product 3D Printer Filament 1KG",
                  "options": [{ "name": "Color", "values": ["{{rawColor}}"] }]
                }
              ]
            }
            """;

        var products = SunluStoreParser.ParseCollection(json);

        Assert.Equal([expectedClean], Assert.Single(products).Colors);
    }

    [Fact]
    public void SplitMaterialVariant_UnknownTitle_FallsBackToRawTitleAsMaterial()
    {
        var (material, variant) = SunluStoreParser.SplitMaterialVariant("Some Brand New SUNLU Product");

        Assert.Equal("Some Brand New SUNLU Product", material);
        Assert.Null(variant);
    }
}
