namespace Spoolbook.FilamentSync.Tests;

public class ColorHexResolverTests
{
    [Theory]
    [InlineData("Black", "#000000")]
    [InlineData("black", "#000000")] // case-insensitive
    [InlineData("Hot Pink", "#FF69B4")] // multi-word exact CSS match
    [InlineData("HotPink", "#FF69B4")] // no-space variant
    public void Resolve_ExactMatch_ReturnsKnownHex(string name, string expectedHex)
    {
        Assert.Equal(expectedHex, ColorHexResolver.Resolve(name));
    }

    [Theory]
    [InlineData("Matte Black", "#000000")]
    [InlineData("Silk Gold", "#FFD700")]
    [InlineData("Translucent Blue", "#0000FF")]
    public void Resolve_StripsFinishQualifier_ThenMatches(string name, string expectedHex)
    {
        Assert.Equal(expectedHex, ColorHexResolver.Resolve(name));
    }

    [Theory]
    [InlineData("Jade White", "#FFFFFF")] // falls back to last word "White"
    [InlineData("Pumpkin Orange", "#FFA500")] // falls back to "Orange"
    [InlineData("Guacamole Green", "#008000")] // falls back to "Green"
    public void Resolve_NoExactMatch_FallsBackToRecognizableWord(string name, string expectedHex)
    {
        Assert.Equal(expectedHex, ColorHexResolver.Resolve(name));
    }

    [Fact]
    public void Resolve_NoRecognizableWord_ReturnsNull()
    {
        Assert.Null(ColorHexResolver.Resolve("Zxqvyy Flumphdoodle"));
    }
}
