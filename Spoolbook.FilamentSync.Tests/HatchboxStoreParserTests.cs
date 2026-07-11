namespace Spoolbook.FilamentSync.Tests;

public class HatchboxStoreParserTests
{
    [Fact]
    public void FilterRealProducts_ExcludesNonFilamentAndOneOffs()
    {
        var titles = new[]
        {
            "BLACK PLA FILAMENT - 1.75MM, 1KG SPOOL",
            "Yellow 8K 3D Printer Resin PRO - 405nm, 1000ml Bottle",
            "ABS 3D PEN FILAMENT SAMPLE PACK",
            "HATCHBOX T Shirt for Men",
            "Cleaning Filament - 1.75MM (.44 lbs)",
            "Exclusive Release - Temp Change Filament",
            "E-Gift Cards",
        };

        var result = HatchboxStoreParser.FilterRealProducts(titles);

        Assert.Equal(["BLACK PLA FILAMENT - 1.75MM, 1KG SPOOL"], result);
    }

    [Theory]
    [InlineData("GOLD PETG FILAMENT - 1.75MM, 1KG SPOOL", "PETG", null, "Gold")]
    [InlineData("BLACK PLA FILAMENT - 1.75MM, 1KG SPOOL", "PLA", null, "Black")]
    [InlineData("BLACK PLA FILAMENT REFILL & RELOADABLE SPOOL - 1.75MM, 1KG SPOOL", "PLA", null, "Black")]
    [InlineData("BABY BLUE TPU FILAMENT - 1.75MM, 1KG SPOOL (SHORE 95A)", "TPU", null, "Baby Blue")]
    [InlineData("WHITE PA FILAMENT - 1.75MM, 1KG SPOOL", "Nylon", null, "White")]
    [InlineData("TRANSPARENT WHITE PC FILAMENT - 1.75MM, 1KG SPOOL", "PC", "Transparent", "White")]
    // mid-qualifier (between color and material)
    [InlineData("GREEN PERFORMANCE PLA FILAMENT - 1.75MM, 1KG SPOOL", "PLA", "Performance", "Green")]
    [InlineData("BLACK PAINT FREE ABS FILAMENT - 1.75MM, 1KG SPOOL", "ABS", "Paint Free", "Black")]
    [InlineData("BLACK RAPID PETG FILAMENT – 1.75MM, 1KG SPOOL", "PETG", "Rapid", "Black")]
    [InlineData("ASH GRAY MATTE PLA FILAMENT - 1.75MM, 1KG SPOOL", "PLA", "Matte", "Ash Gray")]
    // post-qualifier (after material)
    [InlineData("BLACK PLA MAX V2 FILAMENT - 1.75MM, 1KG SPOOL", "PLA", "Max V2", "Black")]
    [InlineData("BLACK PLA PRO+ FILAMENT - 1.75MM, 1KG SPOOL", "PLA", "Pro+", "Black")]
    // prefix-qualifier (before color)
    [InlineData("SILK BLACK PLA FILAMENT - 1.75MM, 1KG SPOOL", "PLA", "Silk", "Black")]
    [InlineData("METALLIC FINISH GOLD PLA FILAMENT - 1.75MM, 1KG SPOOL", "PLA", "Metallic Finish", "Gold")]
    [InlineData("STONE GRANITE ROCK PLA FILAMENT - 1.75MM, 1KG SPOOL", "PLA", "Stone", "Granite Rock")]
    // stacked qualifiers
    [InlineData("STONE GRAY MATTE PLA FILAMENT - 1.75MM, 1KG SPOOL", "PLA", "Matte Stone", "Gray")]
    // no color word at all -> falls back to Natural
    [InlineData("CARBON FIBER PLA FILAMENT - 1.75MM, 1KG SPOOL", "PLA", "Carbon Fiber", "Natural")]
    [InlineData("TEMPERATURE COLOR CHANGING PLA FILAMENT - 1.75MM, 1KG SPOOL", "PLA", "Temperature Color Changing", "Natural")]
    [InlineData("GLOW IN THE DARK PLA FILAMENT - 1.75MM, 1KG SPOOL", "PLA", "Glow In The Dark", "Natural")]
    [InlineData("GLOW IN THE DARK BLUE PLA FILAMENT - 1.75MM, 1KG SPOOL", "PLA", "Glow In The Dark", "Blue")]
    [InlineData("UV COLOR CHANGING PURPLE PLA FILAMENT - 1.75MM, 1KG SPOOL", "PLA", "UV Color Changing", "Purple")]
    public void ParseProductTitle_ExtractsMaterialVariantColor(string title, string material, string? variant, string color)
    {
        var (m, v, c) = HatchboxStoreParser.ParseProductTitle(title);
        Assert.Equal(material, m);
        Assert.Equal(variant, v);
        Assert.Equal(color, c);
    }
}
