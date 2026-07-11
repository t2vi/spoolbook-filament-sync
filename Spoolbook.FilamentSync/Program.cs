using System.Text.Json;
using Spoolbook.FilamentSync;

if (args.Length < 2)
{
    Console.Error.WriteLine("Usage: Spoolbook.FilamentSync <output-json-path> <all|bambu|esun|elegoo|sunlu|polymaker|prusament|overture|creality|colorfabb|fillamentum|protopasta>");
    return 1;
}

var outputPath = args[0];
var source = args[1];

var entries = source switch
{
    "all" => await SyncAllAsync(),
    "bambu" => await SyncBambuAsync(),
    "esun" => await SyncEsunAsync(),
    "elegoo" => await SyncElegooAsync(),
    "sunlu" => await SyncSunluAsync(),
    "polymaker" => await SyncPolymakerAsync(),
    "prusament" => await SyncPrusamentAsync(),
    "overture" => await SyncOvertureAsync(),
    "creality" => await SyncCrealityAsync(),
    "colorfabb" => await SyncColorfabbAsync(),
    "fillamentum" => await SyncFillamentumAsync(),
    "protopasta" => await SyncProtopastaAsync(),
    _ => null
};

if (entries is null)
{
    Console.Error.WriteLine($"Unknown source '{source}'. Expected 'all', 'bambu', 'esun', 'elegoo', 'sunlu', 'polymaker', 'prusament', 'overture', 'creality', 'colorfabb', 'fillamentum', or 'protopasta'.");
    return 1;
}

// Multi-pack/bundle SKUs (e.g. "PLA-Basic 4 Rolls") re-list the same product's colors under
// the same Material/Variant as the single-roll listing — dedupe rather than maintain a
// growing per-site blocklist of bundle slugs.
var deduped = entries.DistinctBy(e => (e.Brand, e.Material, e.Variant, e.Color)).ToList();

await File.WriteAllTextAsync(outputPath, JsonSerializer.Serialize(deduped, new JsonSerializerOptions { WriteIndented = true }));
Console.WriteLine($"Wrote {deduped.Count} entries to {outputPath}");
return 0;

async Task<List<FilamentSyncEntry>> SyncAllAsync()
{
    var all = new List<FilamentSyncEntry>();
    all.AddRange(await SyncBambuAsync());
    all.AddRange(await SyncEsunAsync());
    all.AddRange(await SyncElegooAsync());
    all.AddRange(await SyncSunluAsync());
    all.AddRange(await SyncPolymakerAsync());
    all.AddRange(await SyncPrusamentAsync());
    all.AddRange(await SyncOvertureAsync());
    all.AddRange(await SyncCrealityAsync());
    all.AddRange(await SyncColorfabbAsync());
    all.AddRange(await SyncFillamentumAsync());
    all.AddRange(await SyncProtopastaAsync());
    return all;
}

async Task<List<FilamentSyncEntry>> SyncBambuAsync()
{
    using var client = new BambuStoreClient();

    Console.WriteLine("Fetching Bambu Lab filament listing...");
    var listingHtml = await client.FetchListingAsync();
    // Bundle/multi-pack SKUs re-sell existing PLA Basic colors under a packaging-descriptor
    // "variant" (e.g. "Basic Refill Pack 10 Rolls") rather than a real product variant — skip
    // them so they don't create near-duplicate catalog rows for colors already captured.
    var skipSlugs = new HashSet<string> { "pla-basic-refill-bundle", "pla-basic-refill-pack" };
    var slugs = BambuStoreParser.ExtractProductSlugs(listingHtml).Where(s => !skipSlugs.Contains(s)).ToList();
    Console.WriteLine($"Found {slugs.Count} products.");

    var result = new List<FilamentSyncEntry>();

    foreach (var slug in slugs)
    {
        Console.WriteLine($"Fetching {slug}...");
        string productHtml;
        try
        {
            productHtml = await client.FetchProductAsync(slug);
        }
        catch (HttpRequestException ex)
        {
            Console.Error.WriteLine($"Skipping {slug}: {ex.Message}");
            continue;
        }

        var page = BambuStoreParser.ParseProductPage(productHtml);
        if (page.Name is null || page.Colors.Count == 0) continue;

        var (material, variant) = BambuStoreParser.SplitMaterialVariant(page.Name);
        result.AddRange(page.Colors.Select(color =>
            new FilamentSyncEntry("Bambu Lab", material, variant, BambuStoreParser.StripMaterialPrefix(color, material))));
    }

    return result;
}

async Task<List<FilamentSyncEntry>> SyncEsunAsync()
{
    using var client = new EsunStoreClient();

    // Multi-roll/bundle SKUs (e.g. "PLA-Basic 4 Rolls", "3KG Spool") encode pack contents as
    // pseudo-"colors" in their Color picker (e.g. "Black+Cold White" for a 2-roll mixed pack) —
    // not real single-spool colors, so skip the whole slug rather than the colors it emits.
    var bundleSlugPattern = new System.Text.RegularExpressions.Regex(
        "rolls|spool|combo|bundle|kit|resin|box|vacuum|swatch", System.Text.RegularExpressions.RegexOptions.IgnoreCase);

    var slugs = new List<string>();
    for (var pageNum = 1; pageNum <= 20; pageNum++)
    {
        Console.WriteLine($"Fetching eSUN listing page {pageNum}...");
        var listingHtml = await client.FetchListingAsync(pageNum);
        var pageSlugs = EsunStoreParser.ExtractProductSlugs(listingHtml);
        if (pageSlugs.Count == 0) break;
        foreach (var slug in pageSlugs)
            if (!slugs.Contains(slug) && !bundleSlugPattern.IsMatch(slug)) slugs.Add(slug);
    }
    Console.WriteLine($"Found {slugs.Count} products.");

    var result = new List<FilamentSyncEntry>();

    foreach (var slug in slugs)
    {
        Console.WriteLine($"Fetching {slug}...");
        string productHtml;
        try
        {
            productHtml = await client.FetchProductAsync(slug);
        }
        catch (HttpRequestException ex)
        {
            Console.Error.WriteLine($"Skipping {slug}: {ex.Message}");
            continue;
        }

        // ParseProductPage returns a null Name for non-filament products (e.g. dry boxes,
        // accessories) that share the same collection listing as the actual filaments.
        var page = EsunStoreParser.ParseProductPage(productHtml);
        if (page.Name is null || page.Colors.Count == 0) continue;

        var (material, variant) = EsunStoreParser.SplitMaterialVariant(page.Name);
        // "eSpool+" is a reusable-spool-holder upsell eSUN embeds directly in the Color
        // picker on some product pages (e.g. pla-refilament) — not a real color.
        result.AddRange(page.Colors.Where(c => c != "eSpool+").Select(color =>
            new FilamentSyncEntry("eSUN", material, variant, color)));
    }

    return result;
}

async Task<List<FilamentSyncEntry>> SyncElegooAsync()
{
    using var client = new ElegooStoreClient();

    Console.WriteLine("Fetching Elegoo filament collection...");
    var json = await client.FetchCollectionAsync();
    var products = ElegooStoreParser.ParseCollection(json);
    Console.WriteLine($"Found {products.Count} products.");

    var result = new List<FilamentSyncEntry>();

    foreach (var product in products)
    {
        var (material, variant) = ElegooStoreParser.SplitMaterialVariant(product.Title);
        result.AddRange(product.Colors.Select(color =>
            new FilamentSyncEntry("Elegoo", material, variant, color)));
    }

    return result;
}

async Task<List<FilamentSyncEntry>> SyncSunluAsync()
{
    using var client = new SunluStoreClient();

    Console.WriteLine("Fetching SUNLU filament collection...");
    var json = await client.FetchCollectionAsync();
    var products = SunluStoreParser.ParseCollection(json);
    Console.WriteLine($"Found {products.Count} products.");

    var result = new List<FilamentSyncEntry>();

    foreach (var product in products)
    {
        var (material, variant) = SunluStoreParser.SplitMaterialVariant(product.Title);
        result.AddRange(product.Colors.Select(color =>
            new FilamentSyncEntry("SUNLU", material, variant, color)));
    }

    return result;
}

async Task<List<FilamentSyncEntry>> SyncPolymakerAsync()
{
    using var client = new PolymakerStoreClient();

    Console.WriteLine("Fetching Polymaker filament collection...");
    var json = await client.FetchCollectionAsync();
    var products = PolymakerStoreParser.ParseCollection(json);
    Console.WriteLine($"Found {products.Count} products.");

    var result = new List<FilamentSyncEntry>();

    foreach (var product in products)
    {
        var (material, variant) = PolymakerStoreParser.SplitMaterialVariant(product.Title);
        result.AddRange(product.Colors.Select(color =>
            new FilamentSyncEntry("Polymaker", material, variant, color)));
    }

    return result;
}

async Task<List<FilamentSyncEntry>> SyncPrusamentAsync()
{
    using var client = new PrusamentStoreClient();

    Console.WriteLine("Fetching Prusament filament collection...");
    var json = await client.FetchCollectionAsync();
    var names = PrusamentStoreParser.ParseCollection(json);
    Console.WriteLine($"Found {names.Count} products.");

    var result = new List<FilamentSyncEntry>();

    foreach (var name in names)
    {
        var (material, variant, color) = PrusamentStoreParser.ParseProductName(name);
        result.Add(new FilamentSyncEntry("Prusament", material, variant, color));
    }

    return result;
}

async Task<List<FilamentSyncEntry>> SyncOvertureAsync()
{
    using var client = new OvertureStoreClient();

    Console.WriteLine("Fetching Overture filament collection...");
    var json = await client.FetchCollectionAsync();
    var products = OvertureStoreParser.ParseCollection(json);
    Console.WriteLine($"Found {products.Count} products.");

    var result = new List<FilamentSyncEntry>();

    foreach (var product in products)
    {
        var (material, variant) = OvertureStoreParser.SplitMaterialVariant(product.ProductType);
        result.AddRange(product.Colors.Select(color =>
            new FilamentSyncEntry("Overture", material, variant, color)));
    }

    return result;
}

async Task<List<FilamentSyncEntry>> SyncCrealityAsync()
{
    using var client = new CrealityStoreClient();

    Console.WriteLine("Fetching Creality filament collection...");
    var json = await client.FetchCollectionAsync();
    var products = CrealityStoreParser.ParseCollection(json);
    Console.WriteLine($"Found {products.Count} products.");

    var result = new List<FilamentSyncEntry>();

    foreach (var product in products)
    {
        var (material, variant) = CrealityStoreParser.SplitMaterialVariant(product.Title);
        result.AddRange(product.Colors.Select(color =>
            new FilamentSyncEntry("Creality", material, variant, color)));
    }

    return result;
}

async Task<List<FilamentSyncEntry>> SyncFillamentumAsync()
{
    using var client = new FillamentumStoreClient();

    Console.WriteLine("Fetching Fillamentum filament collection (paginated)...");
    var products = await client.FetchAllProductsAsync();
    Console.WriteLine($"Found {products.Count} products.");

    var result = new List<FilamentSyncEntry>();

    foreach (var product in products)
    {
        var (material, variant, colors) = FillamentumStoreParser.ParseProduct(product);
        result.AddRange(colors.Select(color => new FilamentSyncEntry("Fillamentum", material, variant, color)));
    }

    return result;
}

async Task<List<FilamentSyncEntry>> SyncProtopastaAsync()
{
    using var client = new ProtopastaStoreClient();

    Console.WriteLine("Fetching Protopasta filament collection...");
    var json = await client.FetchCollectionAsync();
    var titles = ProtopastaStoreParser.ParseCollection(json);
    var realTitles = ProtopastaStoreParser.FilterRealProducts(titles);
    Console.WriteLine($"Found {realTitles.Count} products.");

    var result = new List<FilamentSyncEntry>();

    foreach (var title in realTitles)
    {
        var (material, variant, color) = ProtopastaStoreParser.ParseProductTitle(title);
        result.Add(new FilamentSyncEntry("Protopasta", material, variant, color));
    }

    return result;
}

async Task<List<FilamentSyncEntry>> SyncColorfabbAsync()
{
    using var client = new ColorfabbStoreClient();

    Console.WriteLine("Fetching colorFabb filament listing (paginated)...");
    var titles = await client.FetchAllProductTitlesAsync();
    var realTitles = ColorfabbStoreParser.FilterRealProducts(titles);
    Console.WriteLine($"Found {realTitles.Count} products.");

    var result = new List<FilamentSyncEntry>();

    foreach (var title in realTitles)
    {
        var (material, variant, color) = ColorfabbStoreParser.ParseProductTitle(title);
        result.Add(new FilamentSyncEntry("Colorfabb", material, variant, color));
    }

    return result;
}
