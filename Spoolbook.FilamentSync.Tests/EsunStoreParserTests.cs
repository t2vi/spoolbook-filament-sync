namespace Spoolbook.FilamentSync.Tests;

public class EsunStoreParserTests
{
    [Fact]
    public void ExtractProductSlugs_ReturnsUniqueSlugsInOrder()
    {
        var html = """
            <a href="/products/epla">PLA-Basic</a>
            <a href="/products/pla-pro">PLA+</a>
            <a href="/products/epla">PLA-Basic</a>
            <a href="/collections/resin">not a product</a>
            """;

        var slugs = EsunStoreParser.ExtractProductSlugs(html);

        Assert.Equal(["epla", "pla-pro"], slugs);
    }

    [Fact]
    public void ParseProductPage_ExtractsNameAndDedupedColors()
    {
        var html = """
            <h1 itemprop="name" class="detail_name themes_products_title" >eSUN PLA-Basic 1.75mm 3D Filament 1KG</h1>
            <li class="attr_show attr_show_list" name="Color" data-position="1" data-type="color" data-picture="0">
                <div class="attr_box">
                    <div value="Black" data="{&quot;Price&quot;:0}" class="btn_attr" data-title="Black" data-is-picture="0">
                        <span class="attr_color" style="background-color:#393d47"></span>
                    </div>
                    <div value="Cold White" data="{&quot;Price&quot;:0}" class="btn_attr" data-title="Cold White" data-is-picture="0">
                        <span class="attr_color" style="background-color:#f8f8ff"></span>
                    </div>
                    <div value="Black" data="{&quot;Price&quot;:0}" class="btn_attr" data-title="Black" data-is-picture="0">
                        <span class="attr_color" style="background-color:#393d47"></span>
                    </div>
                </div>
            </li>
            """;

        var page = EsunStoreParser.ParseProductPage(html);

        Assert.Equal("PLA-Basic", page.Name);
        Assert.Equal(["Black", "Cold White"], page.Colors);
    }

    [Fact]
    public void ParseProductPage_IgnoresNonColorAttrOptions()
    {
        var html = """
            <h1 itemprop="name" class="detail_name themes_products_title" >eSUN PETG 1.75mm 3D Filament 1KG</h1>
            <li class="attr_show attr_show_list" name="Ship From" data-type="text">
                <div class="attr_box">
                    <div value="3" class="btn_attr" data-title="US">US</div>
                </div>
            </li>
            <li class="attr_show attr_show_list" name="Color" data-position="1" data-type="color" data-picture="0">
                <div class="attr_box">
                    <div value="Black" class="btn_attr" data-title="Black" data-is-picture="0">
                        <span class="attr_color" style="background-color:#000000"></span>
                    </div>
                </div>
            </li>
            """;

        var page = EsunStoreParser.ParseProductPage(html);

        Assert.Equal(["Black"], page.Colors);
    }

    [Fact]
    public void ParseProductPage_PictureBasedSwatchesHaveNoHexSpanButStillCountAsColors()
    {
        var html = """
            <h1 itemprop="name" class="detail_name themes_products_title" >eSUN PETG 1.75mm 3D Filament 1KG</h1>
            <li class="attr_show attr_show_list" name="Color" data-position="1" data-type="picture" data-picture="0">
                <div class="attr_box">
                    <div value="Solid Black" data="{&quot;Price&quot;:0}" class="btn_attr" data-title="Solid Black" data-is-picture="1">
                        <img src="thumb.jpg"/>
                    </div>
                    <div value="Solid White" data="{&quot;Price&quot;:0}" class="btn_attr" data-title="Solid White" data-is-picture="1">
                        <img src="thumb2.jpg"/>
                    </div>
                </div>
            </li>
            """;

        var page = EsunStoreParser.ParseProductPage(html);

        Assert.Equal(["Solid Black", "Solid White"], page.Colors);
    }

    [Fact]
    public void ParseProductPage_IgnoresBundleAndAddonPickersOutsideColorBlock()
    {
        var html = """
            <h1 itemprop="name" class="detail_name themes_products_title" >eSUN PLA-Basic 1.75mm 3D Filament 1KG</h1>
            <li class="attr_show attr_show_list" name="Color" data-position="1" data-type="color" data-picture="0">
                <div class="attr_box">
                    <div value="Black" class="btn_attr" data-title="Black" data-is-picture="0">
                        <span class="attr_color" style="background-color:#000000"></span>
                    </div>
                </div>
            </li>
            <li class="attr_show attr_show_list" name="Bundle" data-type="bundle">
                <div class="attr_box">
                    <div value="Black 4rolls" class="btn_attr" data-title="Black 4rolls" data-is-picture="1">
                        <img src="bundle.jpg"/>
                    </div>
                    <div value="Classic Bundle" class="btn_attr" data-title="Classic Bundle" data-is-picture="1">
                        <img src="bundle2.jpg"/>
                    </div>
                </div>
            </li>
            <div class="ajax_you_make_also_like" attrid="1">
                <div value="eSpool+" class="btn_attr" data-title="eSpool+" data-is-picture="1">
                    <img src="addon.jpg"/>
                </div>
            </div>
            """;

        var page = EsunStoreParser.ParseProductPage(html);

        Assert.Equal(["Black"], page.Colors);
    }

    [Fact]
    public void ParseProductPage_NonFilamentProduct_ReturnsNullName()
    {
        var html = """
            <h1 itemprop="name" class="detail_name themes_products_title" >eSUN Dry Box Lite</h1>
            """;

        var page = EsunStoreParser.ParseProductPage(html);

        Assert.Null(page.Name);
    }

    [Theory]
    [InlineData("PLA-Basic", "PLA", "Basic")]
    [InlineData("PLA+", "PLA", "Plus")]
    [InlineData("PETG", "PETG", null)]
    [InlineData("ABS+", "ABS", "Plus")]
    [InlineData("TPU-95A", "TPU", "95A")]
    [InlineData("PLA-Silk Magic", "PLA", "Silk Magic")]
    [InlineData("PLA+ Refilament", "PLA", "Plus Refilament")]
    public void SplitMaterialVariant_SplitsOnPlusOrDash(string name, string expectedMaterial, string? expectedVariant)
    {
        var (material, variant) = EsunStoreParser.SplitMaterialVariant(name);

        Assert.Equal(expectedMaterial, material);
        Assert.Equal(expectedVariant, variant);
    }
}
