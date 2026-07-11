namespace Spoolbook.FilamentSync.Tests;

public class OvertureStoreParserTests
{
    [Fact]
    public void ParseCollection_ExtractsColorsAndNormalizesHyphenatedDualColorNames()
    {
        var json = """
            {
              "products": [
                {
                  "title": "Overture Matte PLA Dual Colors 3D Printer Filament 1.75mm",
                  "product_type": "3D Printer Filament > PLA > MATTE PLA",
                  "options": [{ "name": "Color", "values": ["Black-White", "Silk Tiger Eye"] }]
                },
                {
                  "title": "Overture PLA Refill 3D Printer Filament 1.75mm",
                  "product_type": "3D Printer Filament > PLA > PLA",
                  "options": [{ "name": "Title", "values": ["Default Title"] }]
                }
              ]
            }
            """;

        var products = OvertureStoreParser.ParseCollection(json);

        var product = Assert.Single(products);
        Assert.Equal(["Black+White", "Silk Tiger Eye"], product.Colors);
    }

    [Theory]
    [InlineData("3D Printer Filament > PLA > PLA", "PLA", null)]
    [InlineData("3D Printer Filament > PLA > MATTE PLA", "PLA", "Matte")]
    [InlineData("3D Printer Filament > PLA > PLA PROFESSIONAL", "PLA", "Professional")]
    [InlineData("3D Printer Filament > PLA+> PLA+", "PLA+", null)]
    [InlineData("3D Printer Filament > PLA > SUPER PLA+", "PLA", "Super")]
    [InlineData("3D Printer Filament > TPU > HIGH SPEED TPU", "TPU", "High Speed")]
    [InlineData("3D Printer Filament > Nylon > EASY NYLON", "Nylon", "Easy")]
    [InlineData("3D Printer Filament > PC > PC PROFESSIONAL", "PC", "Professional")]
    public void SplitMaterialVariant_SplitsHierarchicalProductType(string productType, string expectedMaterial, string? expectedVariant)
    {
        var (material, variant) = OvertureStoreParser.SplitMaterialVariant(productType);

        Assert.Equal(expectedMaterial, material);
        Assert.Equal(expectedVariant, variant);
    }
}
