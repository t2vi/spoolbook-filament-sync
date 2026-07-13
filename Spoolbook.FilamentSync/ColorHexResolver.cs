namespace Spoolbook.FilamentSync;

// Best-effort hex for a scraped color name, so the app doesn't show a flat #CCCCCC swatch for
// everything. Not a live web search per lookup (unreliable/slow for ~1000+ names in a CI run)
// — instead the standard CSS/X11 named-color table (the same answer a web search for "hex for
// hot pink" gives) plus a small supplementary table of common filament/marketing color words,
// matched exactly first, then via word-level fallback for compound marketing names like
// "Guacamole Green" (falls back to "Green").
public static class ColorHexResolver
{
    // CSS Color Module Level 4 extended keywords — the standard reference every browser and
    // "what's the hex for X" search ultimately uses.
    private static readonly Dictionary<string, string> CssColors = new(StringComparer.OrdinalIgnoreCase)
    {
        ["aliceblue"] = "#F0F8FF", ["antiquewhite"] = "#FAEBD7", ["aqua"] = "#00FFFF",
        ["aquamarine"] = "#7FFFD4", ["azure"] = "#F0FFFF", ["beige"] = "#F5F5DC",
        ["bisque"] = "#FFE4C4", ["black"] = "#000000", ["blanchedalmond"] = "#FFEBCD",
        ["blue"] = "#0000FF", ["blueviolet"] = "#8A2BE2", ["brown"] = "#A52A2A",
        ["burlywood"] = "#DEB887", ["cadetblue"] = "#5F9EA0", ["chartreuse"] = "#7FFF00",
        ["chocolate"] = "#D2691E", ["coral"] = "#FF7F50", ["cornflowerblue"] = "#6495ED",
        ["cornsilk"] = "#FFF8DC", ["crimson"] = "#DC143C", ["cyan"] = "#00FFFF",
        ["darkblue"] = "#00008B", ["darkcyan"] = "#008B8B", ["darkgoldenrod"] = "#B8860B",
        ["darkgray"] = "#A9A9A9", ["darkgrey"] = "#A9A9A9", ["darkgreen"] = "#006400",
        ["darkkhaki"] = "#BDB76B", ["darkmagenta"] = "#8B008B", ["darkolivegreen"] = "#556B2F",
        ["darkorange"] = "#FF8C00", ["darkorchid"] = "#9932CC", ["darkred"] = "#8B0000",
        ["darksalmon"] = "#E9967A", ["darkseagreen"] = "#8FBC8F", ["darkslateblue"] = "#483D8B",
        ["darkslategray"] = "#2F4F4F", ["darkturquoise"] = "#00CED1", ["darkviolet"] = "#9400D3",
        ["deeppink"] = "#FF1493", ["deepskyblue"] = "#00BFFF", ["dimgray"] = "#696969",
        ["dodgerblue"] = "#1E90FF", ["firebrick"] = "#B22222", ["floralwhite"] = "#FFFAF0",
        ["forestgreen"] = "#228B22", ["fuchsia"] = "#FF00FF", ["gainsboro"] = "#DCDCDC",
        ["ghostwhite"] = "#F8F8FF", ["gold"] = "#FFD700", ["goldenrod"] = "#DAA520",
        ["gray"] = "#808080", ["grey"] = "#808080", ["green"] = "#008000",
        ["greenyellow"] = "#ADFF2F", ["honeydew"] = "#F0FFF0", ["hotpink"] = "#FF69B4",
        ["indianred"] = "#CD5C5C", ["indigo"] = "#4B0082", ["ivory"] = "#FFFFF0",
        ["khaki"] = "#F0E68C", ["lavender"] = "#E6E6FA", ["lavenderblush"] = "#FFF0F5",
        ["lawngreen"] = "#7CFC00", ["lemonchiffon"] = "#FFFACD", ["lightblue"] = "#ADD8E6",
        ["lightcoral"] = "#F08080", ["lightcyan"] = "#E0FFFF", ["lightgoldenrodyellow"] = "#FAFAD2",
        ["lightgray"] = "#D3D3D3", ["lightgrey"] = "#D3D3D3", ["lightgreen"] = "#90EE90",
        ["lightpink"] = "#FFB6C1", ["lightsalmon"] = "#FFA07A", ["lightseagreen"] = "#20B2AA",
        ["lightskyblue"] = "#87CEFA", ["lightslategray"] = "#778899", ["lightsteelblue"] = "#B0C4DE",
        ["lightyellow"] = "#FFFFE0", ["lime"] = "#00FF00", ["limegreen"] = "#32CD32",
        ["linen"] = "#FAF0E6", ["magenta"] = "#FF00FF", ["maroon"] = "#800000",
        ["mediumaquamarine"] = "#66CDAA", ["mediumblue"] = "#0000CD", ["mediumorchid"] = "#BA55D3",
        ["mediumpurple"] = "#9370DB", ["mediumseagreen"] = "#3CB371", ["mediumslateblue"] = "#7B68EE",
        ["mediumspringgreen"] = "#00FA9A", ["mediumturquoise"] = "#48D1CC", ["mediumvioletred"] = "#C71585",
        ["midnightblue"] = "#191970", ["mintcream"] = "#F5FFFA", ["mistyrose"] = "#FFE4E1",
        ["moccasin"] = "#FFE4B5", ["navajowhite"] = "#FFDEAD", ["navy"] = "#000080",
        ["oldlace"] = "#FDF5E6", ["olive"] = "#808000", ["olivedrab"] = "#6B8E23",
        ["orange"] = "#FFA500", ["orangered"] = "#FF4500", ["orchid"] = "#DA70D6",
        ["palegoldenrod"] = "#EEE8AA", ["palegreen"] = "#98FB98", ["paleturquoise"] = "#AFEEEE",
        ["palevioletred"] = "#DB7093", ["papayawhip"] = "#FFEFD5", ["peachpuff"] = "#FFDAB9",
        ["peru"] = "#CD853F", ["pink"] = "#FFC0CB", ["plum"] = "#DDA0DD",
        ["powderblue"] = "#B0E0E6", ["purple"] = "#800080", ["rebeccapurple"] = "#663399",
        ["red"] = "#FF0000", ["rosybrown"] = "#BC8F8F", ["royalblue"] = "#4169E1",
        ["saddlebrown"] = "#8B4513", ["salmon"] = "#FA8072", ["sandybrown"] = "#F4A460",
        ["seagreen"] = "#2E8B57", ["seashell"] = "#FFF5EE", ["sienna"] = "#A0522D",
        ["silver"] = "#C0C0C0", ["skyblue"] = "#87CEEB", ["slateblue"] = "#6A5ACD",
        ["slategray"] = "#708090", ["snow"] = "#FFFAFA", ["springgreen"] = "#00FF7F",
        ["steelblue"] = "#4682B4", ["tan"] = "#D2B48C", ["teal"] = "#008080",
        ["thistle"] = "#D8BFD8", ["tomato"] = "#FF6347", ["turquoise"] = "#40E0D0",
        ["violet"] = "#EE82EE", ["wheat"] = "#F5DEB3", ["white"] = "#FFFFFF",
        ["whitesmoke"] = "#F5F5F5", ["yellow"] = "#FFFF00", ["yellowgreen"] = "#9ACD32",
    };

    // Common color words seen repeatedly across scraped filament brands that aren't standard
    // CSS keywords — approximate but far better than flat gray.
    private static readonly Dictionary<string, string> SupplementaryColors = new(StringComparer.OrdinalIgnoreCase)
    {
        ["kraft"] = "#C09A6B", ["rosegold"] = "#B76E79", ["champagne"] = "#F7E7CE",
        ["cocoa"] = "#6B4423", ["cognac"] = "#834333", ["terracotta"] = "#E2725B",
        ["sand"] = "#C2B280", ["nude"] = "#E3BC9A", ["blush"] = "#DE5D83",
        ["mint"] = "#98FF98", ["sage"] = "#9CAF88", ["mustard"] = "#FFDB58",
        ["rust"] = "#B7410E", ["charcoal"] = "#36454F", ["denim"] = "#1560BD",
        ["periwinkle"] = "#CCCCFF", ["mauve"] = "#E0B0FF", ["burgundy"] = "#800020",
        ["emerald"] = "#50C878", ["jade"] = "#00A86B", ["amber"] = "#FFBF00",
        ["bronze"] = "#CD7F32", ["copper"] = "#B87333", ["pewter"] = "#96A8A8",
        ["chrome"] = "#DBE4EB", ["titanium"] = "#878681", ["graphite"] = "#383838",
        ["onyx"] = "#353839", ["pearl"] = "#EAE0C8", ["camel"] = "#C19A6B",
        ["chestnut"] = "#954535", ["mahogany"] = "#C04000", ["walnut"] = "#773F1A",
        ["espresso"] = "#4B3621", ["caramel"] = "#C68E17", ["honey"] = "#EBA937",
        ["saffron"] = "#F4C430", ["marigold"] = "#EAA221", ["tangerine"] = "#F28500",
        ["watermelon"] = "#FC6C85", ["cherry"] = "#DE3163", ["ruby"] = "#E0115F",
        ["garnet"] = "#733635", ["scarlet"] = "#FF2400", ["vermillion"] = "#E34234",
        ["brick"] = "#B44E4E", ["umber"] = "#635147", ["ochre"] = "#CC7722",
        ["grape"] = "#6F2DA8", ["lilac"] = "#C8A2C8", ["lemon"] = "#FFF700",
        ["apricot"] = "#FBCEB1", ["babyblue"] = "#89CFF0", ["natural"] = "#F5F0E1",
        ["clear"] = "#FFFFFF", ["transparent"] = "#FFFFFF", ["cream"] = "#FFFDD0",
        ["blaze"] = "#FF6600", ["flesh"] = "#FFDBAC", ["brass"] = "#B5A642",
        ["clay"] = "#B66A50", ["coffee"] = "#6F4E37", ["forest"] = "#228B22",
        ["rosewood"] = "#65000B", ["oak"] = "#C19A6B", ["birch"] = "#DEB887",
        ["aspen"] = "#E8DCC4", ["maple"] = "#E8B84B", ["granite"] = "#676767",
        ["sandstone"] = "#C2985B", ["limestone"] = "#DCD2C0",
    };

    // Finish/technique/product-line words that modify appearance but aren't themselves a hue —
    // stripped before re-attempting a match (e.g. "Matte Black" -> "Black").
    private static readonly string[] QualifierWords =
    [
        "matte", "silk", "silky", "glossy", "translucent", "transparent", "metallic",
        "glow", "dark", "rainbow", "sparkle", "marble", "opaque", "composite", "fiber",
        "carbon", "wood", "stone", "performance", "pro", "basic", "rapid", "max", "spoolless",
        "premium", "recycled", "smooth", "rigid", "flexible", "reflective", "thermochromic",
    ];

    public static string? Resolve(string name)
    {
        var normalized = Normalize(name);
        if (TryLookup(normalized, out var hex)) return hex;

        var words = name.Split([' ', '-', '_', '(', ')', '+', '/'], StringSplitOptions.RemoveEmptyEntries)
            .Where(w => !QualifierWords.Contains(w, StringComparer.OrdinalIgnoreCase))
            .ToList();

        if (words.Count > 0)
        {
            var strippedNormalized = Normalize(string.Join(' ', words));
            if (strippedNormalized != normalized && TryLookup(strippedNormalized, out hex)) return hex;
        }

        for (var i = words.Count - 1; i >= 0; i--)
            if (TryLookup(Normalize(words[i]), out hex)) return hex;

        return null;
    }

    private static bool TryLookup(string normalized, out string hex) =>
        CssColors.TryGetValue(normalized, out hex!) || SupplementaryColors.TryGetValue(normalized, out hex!);

    private static string Normalize(string s) =>
        new(s.Where(char.IsLetter).ToArray());
}
