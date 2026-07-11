namespace Spoolbook.FilamentSync.Tests;

public class ProtopastaStoreParserTests
{
    [Fact]
    public void ParseListing_ExcludesNonFilamentAndAmbiguousProducts()
    {
        var titles = new[]
        {
            "Black c-Matte PLA",
            "Endless PLA Filament Color Subscription *2026 update*",
            "\"We Keep Us Safe\" by The Whistle Crew",
            "Green Glow-in-the-Dark, Natural",
        };

        var kept = ProtopastaStoreParser.FilterRealProducts(titles);

        Assert.Equal(["Black c-Matte PLA"], kept);
    }

    [Theory]
    [InlineData("Green Quantum Dot HTPLA", "HTPLA", "Quantum Dot", "Green")]
    [InlineData("Yellow Reflective HTPLA", "HTPLA", "Reflective", "Yellow")]
    [InlineData("Fluorescent Yellow Reflective HTPLA", "HTPLA", "Reflective", "Fluorescent Yellow")]
    [InlineData("Gradient Gray Multicolor HTPLA", "HTPLA", "Multicolor", "Gradient Gray")]
    [InlineData("Jurassic Jungle Green c-Matte PLA", "PLA", "c-Matte", "Jurassic Jungle Green")]
    [InlineData("Clear PCTG", "PCTG", null, "Clear")]
    [InlineData("Obsidian HTPLA", "HTPLA", null, "Obsidian")]
    [InlineData("Stef's Rose Gold HTPLA", "HTPLA", null, "Stef's Rose Gold")]
    [InlineData("Black High Strength Carbon Fiber PCTG", "PCTG", "High Strength Carbon Fiber", "Black")]
    [InlineData("Light Gray Carbon Fiber Composite HTPLA", "HTPLA", "Carbon Fiber Composite", "Light Gray")]
    [InlineData("Static Dissipative Carbon Fiber PLA", "PLA", "Static Dissipative Carbon Fiber", "Natural")]
    [InlineData("Copper-filled Metal Composite HTPLA", "HTPLA", "Metal Composite", "Copper")]
    [InlineData("High Density Iron-filled HTPLA", "HTPLA", "High Density", "Iron")]
    [InlineData("Natural High Flow PLA (HFPLA)", "PLA", "High Flow", "Natural")]
    [InlineData("Clear PETG *new low price*", "PETG", null, "Clear")]
    [InlineData("Black PLA - Premium Basic PLA", "PLA", "Premium Basic", "Black")]
    [InlineData("Matte Fiber HTPLA - Daffodil Wood", "HTPLA", "Matte Fiber", "Daffodil Wood")]
    [InlineData("Stainless Steel Metal Composite PLA - Blue color", "PLA", "Stainless Steel Metal Composite", "Blue")]
    [InlineData("Night Before Blue HTPLA with Silver Glitter", "HTPLA", null, "Night Before Blue with Silver Glitter")]
    [InlineData("Texas Tea Black with Gold Glitter", "HTPLA", null, "Texas Tea Black with Gold Glitter")]
    public void ParseProductTitle_SplitsMaterialVariantColor(string title, string expectedMaterial, string? expectedVariant, string expectedColor)
    {
        var (material, variant, color) = ProtopastaStoreParser.ParseProductTitle(title);

        Assert.Equal(expectedMaterial, material);
        Assert.Equal(expectedVariant, variant);
        Assert.Equal(expectedColor, color);
    }
}
