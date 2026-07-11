namespace Spoolbook.FilamentSync.Tests;

public class Slic3DStoreParserTests
{
    [Fact]
    public void ParseListingPage_ExtractsAltTextFromProductImages()
    {
        const string html = """
            <img alt="Slic3D PETG filament Black 1.75mm 1kg" src="a.jpg">
            <img alt="Slic3D PLA Spool-Less White 1.75mm 1kg filament" src="b.jpg">
            <img alt="Not a product" src="c.jpg">
            """;

        var titles = Slic3DStoreParser.ParseListingPage(html);

        Assert.Equal(["Slic3D PETG filament Black 1.75mm 1kg", "Slic3D PLA Spool-Less White 1.75mm 1kg filament"], titles);
    }

    [Theory]
    [InlineData("Slic3D PETG filament Black 1.75mm 1kg", "PETG", null, "Black")]
    [InlineData("Slic3D PETG filament Yellow1.75mm 1kg", "PETG", null, "Yellow")] // real title has no space before size
    [InlineData("Slic3D PLA Spool-Less Apple Green 1.75mm 1kg filament", "PLA", "Spool-Less", "Apple Green")]
    [InlineData("Slic3D PLA Spool-Less Cocoa Brown 1.75mm 1kg filament", "PLA", "Spool-Less", "Cocoa Brown")]
    public void ParseProductTitle_ExtractsMaterialVariantColor(string altText, string material, string? variant, string color)
    {
        var (m, v, c) = Slic3DStoreParser.ParseProductTitle(altText);
        Assert.Equal(material, m);
        Assert.Equal(variant, v);
        Assert.Equal(color, c);
    }
}
