namespace Spoolbook.FilamentSync.Tests;

public class FillamentumStoreParserTests
{
    private const string ListingJson = """
    {
      "products": [
        {
          "title": "PLA Extrafill \"Natural\" | 1 KG | 1.75 mm",
          "product_type": "filament for 3D printing",
          "variants": [{ "title": "1.75 mm / 1 Kg" }]
        },
        {
          "title": "15 m Sample | 1.75 mm | PP 2320",
          "product_type": "filament for 3D printing",
          "variants": [{ "title": "1.75 mm / Natural" }, { "title": "1.75 mm / Black" }]
        },
        {
          "title": "Nylon FX256 + LockPAd",
          "product_type": "filament for 3D printing",
          "variants": [{ "title": "\"Natural\" 1.75 mm" }]
        },
        {
          "title": "PLA Extrafill Tool",
          "product_type": "Tools",
          "variants": [{ "title": "Default Title" }]
        }
      ]
    }
    """;

    [Fact]
    public void ParseCollection_KeepsOnlyRealFilamentProducts()
    {
        var products = FillamentumStoreParser.ParseCollection(ListingJson);

        Assert.Equal(["PLA Extrafill \"Natural\" | 1 KG | 1.75 mm"], products.Select(p => p.Title));
    }

    [Theory]
    [InlineData("PLA Extrafill \"Natural\" | 1 KG | 1.75 mm", "PLA", "Extrafill", new[] { "Natural" })]
    [InlineData("PLA Extrafill \"Traffic White\"", "PLA", "Extrafill", new[] { "Traffic White" })]
    [InlineData("PLA Extrafill \"Everybody's Magenta\" | 1 KG | 1.75 mm", "PLA", "Extrafill", new[] { "Everybody's Magenta" })]
    [InlineData("PLA Extrafill \"Witch Please!\"", "PLA", "Extrafill", new[] { "Witch Please!" })]
    [InlineData("NonOilen® \"Ginger Shot\"", "NonOilen®", null, new[] { "Ginger Shot" })]
    [InlineData("Flexfill TPU 92A \"Luminous Green\"", "TPU", "Flexfill 92A", new[] { "Luminous Green" })]
    [InlineData("Flexfill TPE 90A \"Traffic Black\"", "TPE", "Flexfill 90A", new[] { "Traffic Black" })]
    [InlineData("ASA CF10 Carbon \"Natural\"", "ASA", "CF10 Carbon", new[] { "Natural" })]
    [InlineData("Nylon FX256 \"Sky Blue\"", "Nylon", "FX256", new[] { "Sky Blue" })]
    [InlineData("rePETG Loopfill \"Custom Color\"", "rPETG", "Loopfill", new string[0])]
    public void ParseProduct_QuotedColor_ReturnsMaterialVariantColor(
        string title, string expectedMaterial, string? expectedVariant, string[] expectedColors)
    {
        var product = new FillamentumProduct(title, ["1.75 mm / 1 Kg"]);

        var (material, variant, colors) = FillamentumStoreParser.ParseProduct(product);

        Assert.Equal(expectedMaterial, material);
        Assert.Equal(expectedVariant, variant);
        Assert.Equal(expectedColors, colors);
    }

    [Fact]
    public void ParseProduct_NoQuotedColor_ExtractsColorsFromVariantTitles()
    {
        var product = new FillamentumProduct(
            "Polypropylene PP 2320",
            ["1.75 mm / Natural", "1.75 mm / Black"]);

        var (material, variant, colors) = FillamentumStoreParser.ParseProduct(product);

        Assert.Equal("PP", material);
        Assert.Equal("2320", variant);
        Assert.Equal(["Black", "Natural"], colors);
    }

    [Fact]
    public void ParseProduct_NoQuotedColorAndNoColorVariant_DefaultsToNatural()
    {
        var product = new FillamentumProduct(
            "Nylon AF80 Aramid",
            ["1.75 mm", "2.85 mm"]);

        var (material, variant, colors) = FillamentumStoreParser.ParseProduct(product);

        Assert.Equal("Nylon", material);
        Assert.Equal("AF80 Aramid", variant);
        Assert.Equal(["Natural"], colors);
    }

    [Fact]
    public void ParseProduct_DefaultTitleVariant_DefaultsToNatural()
    {
        var product = new FillamentumProduct(
            "0rCA® | Nylon PA6 + CF10 | 600 g | 1.75",
            ["Default Title"]);

        var (material, variant, colors) = FillamentumStoreParser.ParseProduct(product);

        Assert.Equal("Nylon", material);
        Assert.Equal("0rCA®", variant);
        Assert.Equal(["Natural"], colors);
    }

    [Fact]
    public void ParseProduct_UnknownMaterialLine_UsesTitleAsMaterialWithNullVariant()
    {
        var product = new FillamentumProduct("Mystery Filament \"Blue\"", ["1.75 mm"]);

        var (material, variant, colors) = FillamentumStoreParser.ParseProduct(product);

        Assert.Equal("Mystery Filament", material);
        Assert.Null(variant);
        Assert.Equal(["Blue"], colors);
    }
}
