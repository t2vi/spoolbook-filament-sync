namespace Spoolbook.FilamentSync.Tests;

public class CrealityStoreParserTests
{
    [Fact]
    public void ParseCollection_ExcludesResinAndBundlesAndPackMultiplierPseudoColors()
    {
        var json = """
            {
              "products": [
                {
                  "title": "Hyper Series PLA 3D Printing Filament 1kg",
                  "options": [{ "name": "Color", "values": ["White", "Black", "White*2+Black*2", "2-Pack", "3-Pack(Grey)"] }]
                },
                {
                  "title": "Fast Resin UV Curable Resin 1KG",
                  "options": [{ "name": "Color", "values": ["Grey", "Clear"] }]
                },
                {
                  "title": "8KG Hyper PLA 6 Color Pack 3D Printing Filament",
                  "options": [{ "name": "Title", "values": ["Default Title"] }]
                }
              ]
            }
            """;

        var products = CrealityStoreParser.ParseCollection(json);

        var product = Assert.Single(products);
        Assert.Equal("Hyper Series PLA 3D Printing Filament 1kg", product.Title);
        Assert.Equal(["White", "Black"], product.Colors);
    }

    [Fact]
    public void ParseCollection_NormalizesSimpleTwoToneHyphenatedColorsButNotFancyNames()
    {
        var json = """
            {
              "products": [
                {
                  "title": "CR-Silk 1.75mm PLA 3D Printing Filament 1kg",
                  "options": [{ "name": "Color", "values": ["Golden-silver", "Wild Blossom-Long"] }]
                }
              ]
            }
            """;

        var products = CrealityStoreParser.ParseCollection(json);

        // "Golden-silver" is a real two-tone blend (Capitalized-lowercase single words); "Wild
        // Blossom-Long" is a fancy gradient name where "-Long" means repeat length, not a color.
        Assert.Equal(["Golden+silver", "Wild Blossom-Long"], Assert.Single(products).Colors);
    }

    [Theory]
    [InlineData("Hyper Series PLA 3D Printing Filament 1kg", "PLA", "Hyper Series")]
    [InlineData("Hyper PLA RFID 3D Printing Filament 1kg", "PLA", "Hyper RFID")]
    [InlineData("Hyper Series PLA Carbon Fibre 3D Printing Filament 1kg", "PLA", "Hyper Series Carbon Fibre")]
    [InlineData("CR-Silk 1.75mm PLA 3D Printing Filament 1kg", "PLA", "CR-Silk")]
    [InlineData("HP-TPU 3D Printer Filament 1.75mm 1kg", "TPU", "HP")]
    [InlineData("Hyper PETG-CF RFID 3D Printing Filament 1kg", "PETG-CF", "Hyper RFID")]
    [InlineData("CR PETG 3D Printing Filament 4kg", "PETG", "CR")]
    public void SplitMaterialVariant_StripsNoiseAndKnownMaterialToken(string title, string expectedMaterial, string? expectedVariant)
    {
        var (material, variant) = CrealityStoreParser.SplitMaterialVariant(title);

        Assert.Equal(expectedMaterial, material);
        Assert.Equal(expectedVariant, variant);
    }
}
