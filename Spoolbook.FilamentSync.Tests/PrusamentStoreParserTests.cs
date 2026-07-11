namespace Spoolbook.FilamentSync.Tests;

public class PrusamentStoreParserTests
{
    [Fact]
    public void ParseCollection_SkipsSampleAndMultipackAndNonVariantEntries()
    {
        var json = """
            {
              "data": {
                "category": {
                  "products": {
                    "edges": [
                      { "node": { "name": "Prusament PLA Jet Black 1kg (NFC)" } },
                      { "node": { "name": "Prusament PLA Jet Black 25g sample" } },
                      { "node": { "name": "Prusament TPU 95A Jet Black 500g (NFC) – Multipack 10 pcs" } },
                      { "node": {} }
                    ]
                  }
                }
              }
            }
            """;

        var names = PrusamentStoreParser.ParseCollection(json);

        Assert.Equal(["Prusament PLA Jet Black 1kg (NFC)"], names);
    }

    [Theory]
    [InlineData("Prusament PETG Matte Black 1kg (NFC)", "PETG", null, "Matte Black")]
    [InlineData("Prusament PLA Blend Royal Blue 1kg (NFC)", "PLA", "Blend", "Royal Blue")]
    [InlineData("Prusament Premium PLA Mystic Brown 1kg (NFC)", "PLA", "Premium", "Mystic Brown")]
    [InlineData("Prusament PC Blend Carbon Fiber Black 2kg (NFC)", "PC", "Blend", "Carbon Fiber Black")]
    [InlineData("Prusament PA11 Carbon Fiber Black 800g (NFC)", "PA11", "Carbon Fiber", "Black")]
    [InlineData("Prusament PP Glass Fiber Natural 850g (NFC)", "PP", "Glass Fiber", "Natural")]
    [InlineData("Prusament TPU 95A Jet Black 500g (NFC)", "TPU 95A", null, "Jet Black")]
    [InlineData("Prusament PEI 1010 Natural 500g (NFC)", "PEI 1010", null, "Natural")]
    [InlineData("Prusament PETG Tungsten 75% 1kg (NFC)", "PETG", null, "Tungsten 75%")]
    [InlineData("Prusament PETG Magnetite 40% Grey 1kg (NFC)", "PETG", null, "Magnetite 40% Grey")]
    [InlineData("Prusament PETG Tungsten 75% (100g)", "PETG", null, "Tungsten 75%")]
    [InlineData("Prusament PETG Anthracite Grey 900g Refill (NFC Compatible)", "PETG", null, "Anthracite Grey")]
    [InlineData("Prusament PLA Prusa Orange 1kg (Clearance)", "PLA", null, "Prusa Orange")]
    public void ParseProductName_SplitsMaterialVariantColor(string name, string expectedMaterial, string? expectedVariant, string expectedColor)
    {
        var (material, variant, color) = PrusamentStoreParser.ParseProductName(name);

        Assert.Equal(expectedMaterial, material);
        Assert.Equal(expectedVariant, variant);
        Assert.Equal(expectedColor, color);
    }
}
