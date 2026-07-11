namespace Spoolbook.FilamentSync.Tests;

public class BambuStoreParserTests
{
    [Fact]
    public void ExtractProductSlugs_ReturnsUniqueSortedSlugs()
    {
        var html = """
            <a href="/en/products/pla-basic-filament">PLA Basic</a>
            <a href="/en/products/petg-hf">PETG HF</a>
            <a href="/en/products/pla-basic-filament">PLA Basic</a>
            <a href="/other/not-a-product">nope</a>
            """;

        var slugs = BambuStoreParser.ExtractProductSlugs(html);

        Assert.Equal(["pla-basic-filament", "petg-hf"], slugs);
    }

    [Fact]
    public void ParseProductPage_ExtractsNameAndDedupedColors()
    {
        var html = """
            <title>PLA Basic | Bambu Lab AU Store</title>
            <h1 class="ProductTitle">PLA Basic</h1>
            <li value="Bambu Green (10501)" class="swatch"><img src="a.png"/></li>
            <li value="Mistletoe Green (10502)" class="swatch"><img src="b.png"/></li>
            <li value="Bambu Green (10501)" class="swatch"><img src="a.png"/></li>
            """;

        var page = BambuStoreParser.ParseProductPage(html);

        Assert.Equal("PLA Basic", page.Name);
        Assert.Equal(["Bambu Green", "Mistletoe Green"], page.Colors);
    }

    [Fact]
    public void ParseProductPage_NoColorSwatches_ReturnsEmptyColors()
    {
        var html = """
            <h1>Bambu Reusable Spool</h1>
            """;

        var page = BambuStoreParser.ParseProductPage(html);

        Assert.Empty(page.Colors);
    }

    [Theory]
    [InlineData("ABS Orange", "ABS", "Orange")]
    [InlineData("ABS", "ABS", "ABS")]
    [InlineData("Black", "ABS", "Black")]
    public void StripMaterialPrefix_RemovesLeadingMaterialNameIfPresent(string color, string material, string expected)
    {
        Assert.Equal(expected, BambuStoreParser.StripMaterialPrefix(color, material));
    }

    [Theory]
    [InlineData("PLA Basic", "PLA", "Basic")]
    [InlineData("PLA Basic Gradient", "PLA", "Basic Gradient")]
    [InlineData("PETG HF", "PETG", "HF")]
    [InlineData("ASA-CF", "ASA-CF", null)]
    [InlineData("ABS", "ABS", null)]
    [InlineData("TPU For AMS", "TPU", "For AMS")]
    public void SplitMaterialVariant_SplitsOnFirstSpace(string name, string expectedMaterial, string? expectedVariant)
    {
        var (material, variant) = BambuStoreParser.SplitMaterialVariant(name);

        Assert.Equal(expectedMaterial, material);
        Assert.Equal(expectedVariant, variant);
    }
}
