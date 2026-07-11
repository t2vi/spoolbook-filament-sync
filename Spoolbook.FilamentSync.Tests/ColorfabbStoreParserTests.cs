namespace Spoolbook.FilamentSync.Tests;

public class ColorfabbStoreParserTests
{
    [Fact]
    public void ParseListing_ExcludesThirdPartyRebrandsBundlesAndSamples()
    {
        var titles = new[]
        {
            "VARIOSHORE TPU BLACK",
            "Luvocom 3F PET CF 9780 BK",
            "IGUS IGLIDUR I150",
            "LW-PLA VALUE PACK",
            "Milky White  PLAQUE SAMPLE",
        };

        var kept = ColorfabbStoreParser.FilterRealProducts(titles);

        Assert.Equal(["VARIOSHORE TPU BLACK"], kept);
    }

    [Theory]
    [InlineData("Blue green RAL 6004", "PLA", "RAL", "Blue green RAL 6004")]
    [InlineData("Smokey Black", "PLA", "RAL", "Smokey Black")]
    [InlineData("Milky White", "PLA", "RAL", "Milky White")]
    [InlineData("STEELFILL", "PLA", "steelFill", "Steel")]
    [InlineData("BRONZEFILL", "PLA", "bronzeFill", "Bronze")]
    [InlineData("STONEFILL MOSS GREEN", "PLA", "stoneFill", "Moss Green")]
    [InlineData("VARIOSHORE PROSTHETIC PALE PINK", "TPU", "Varioshore Prosthetic", "Pale Pink")]
    [InlineData("VARIOSHORE TPU BLACK", "TPU", "Varioshore", "BLACK")]
    [InlineData("rPETG Burnt Amber", "rPETG", null, "Burnt Amber")]
    [InlineData("rPLA-Semi-matte-Monumental", "rPLA", "Semi-matte", "Monumental")]
    [InlineData("PLA High Speed Pro Iron Grey", "PLA", "High Speed Pro", "Iron Grey")]
    [InlineData("TPU 95A BLUE", "TPU 95A", null, "BLUE")]
    [InlineData("LW-PLA-HT DARK GRAY", "LW-PLA-HT", null, "DARK GRAY")]
    [InlineData("PLA/PHA VIOLET TRANSPARENT", "PLA/PHA", null, "VIOLET TRANSPARENT")]
    [InlineData("allPHA WHITE", "allPHA", null, "WHITE")]
    [InlineData("NGEN_FLEX DARK GRAY", "nGen Flex", null, "DARK GRAY")]
    [InlineData("PETG ECONOMY CLEAR", "PETG", "Economy", "CLEAR")]
    [InlineData("PA Blue Metal Detectable", "PA", "Metal Detectable", "Blue")]
    [InlineData("nGen-CF10", "nGen-CF10", null, "Natural")]
    [InlineData("XT-CF20", "XT-CF20", null, "Natural")]
    [InlineData("PA Neat", "PA", null, "Natural")]
    [InlineData("VARIOSHORE TPU GREEN 1.75 / 4200", "TPU", "Varioshore", "GREEN")]
    public void ParseProductTitle_SplitsMaterialVariantColor(string title, string expectedMaterial, string? expectedVariant, string expectedColor)
    {
        var (material, variant, color) = ColorfabbStoreParser.ParseProductTitle(title);

        Assert.Equal(expectedMaterial, material);
        Assert.Equal(expectedVariant, variant);
        Assert.Equal(expectedColor, color);
    }
}
