use crate::cloak_browser_client::CloakBrowserClient;
use crate::filament_sync_entry::FilamentSyncEntry;
use crate::source::FilamentSource;
use regex::Regex;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::LazyLock;

// Single request only (Shopify's products.json returns the whole collection at once), same
// shape as Elegoo/Sunlu — polymaker.com's bare domain is behind an active Cloudflare bot
// challenge (cf-mitigated: challenge), but us.polymaker.com is a plain Shopify storefront with
// no such wall.
const BASE_URL: &str = "https://us.polymaker.com";

pub struct PolymakerSource;

#[async_trait::async_trait]
impl FilamentSource for PolymakerSource {
    fn name(&self) -> &'static str {
        "polymaker"
    }

    async fn fetch(&self, _cloak: Option<&CloakBrowserClient>) -> Result<Vec<FilamentSyncEntry>, String> {
        let json = fetch_collection().await?;
        let products = parse_collection(&json);

        Ok(products
            .into_iter()
            .flat_map(|product| {
                let (material, variant) = split_material_variant(&product.title);
                product
                    .colors
                    .into_iter()
                    .map(move |color| FilamentSyncEntry::new("Polymaker", &material, variant.clone(), &color))
            })
            .collect())
    }
}

async fn fetch_collection() -> Result<String, String> {
    let url = format!("{BASE_URL}/collections/all/products.json?limit=250");
    let client = reqwest::Client::builder()
        .user_agent(
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0 Safari/537.36",
        )
        .build()
        .map_err(|e| e.to_string())?;
    let response = client.get(url).send().await.map_err(|e| e.to_string())?;
    response.error_for_status().map_err(|e| e.to_string())?.text().await.map_err(|e| e.to_string())
}

#[derive(Debug, PartialEq)]
struct PolymakerProduct {
    title: String,
    colors: Vec<String>,
}

#[derive(Deserialize)]
struct ShopifyCollection {
    products: Vec<ShopifyProduct>,
}

#[derive(Deserialize)]
struct ShopifyProduct {
    title: String,
    product_type: String,
    options: Option<Vec<ShopifyOption>>,
}

#[derive(Deserialize)]
struct ShopifyOption {
    name: String,
    values: Vec<String>,
}

const REAL_FILAMENT_TYPES: &[&str] = &["Polymaker Filament", "Panchroma Filament", "Fiberon Filament"];

fn parse_collection(json: &str) -> Vec<PolymakerProduct> {
    let data: ShopifyCollection = match serde_json::from_str(json) {
        Ok(d) => d,
        Err(_) => return Vec::new(),
    };

    data.products
        .into_iter()
        .filter(|p| REAL_FILAMENT_TYPES.contains(&p.product_type.as_str()))
        // "Panchroma PLA Refill" mixes colors from several sub-lines (Matte/Silk/Gradient/
        // Marble) into one flat list with no way to attribute a color to its real sub-line.
        .filter(|p| !p.title.to_lowercase().contains("refill"))
        // "PolyLite CosPLA"'s "Color" option is actually a formula-variant selector
        // ("Version A - Durability...", "Version B - Sand-ability..."), not real colors.
        .filter(|p| !p.title.to_lowercase().contains("cospla"))
        .filter_map(|p| {
            let colors = p
                .options
                .unwrap_or_default()
                .into_iter()
                .find(|o| o.name == "Color")
                .map(|o| o.values)
                .unwrap_or_default();
            if colors.is_empty() {
                return None;
            }
            Some(PolymakerProduct { title: p.title, colors })
        })
        .collect()
}

const BRAND_LINES: &[&str] = &[
    "Panchroma", "PolyLite", "PolyMax", "PolyFlex", "PolySmooth", "PolySonic",
    "PolyCast", "PolyDissolve", "PolySupport", "PolyMide", "Polymaker",
];

// Longest/most-specific first so e.g. "HT-PLA-GF" matches before "PLA".
const KNOWN_MATERIALS: &[&str] = &[
    "HT-PLA-GF", "HT-PLA", "LW-PLA", "PC-ABS", "PC-FR", "PLA-CF",
    "TPU95-HF", "TPU90", "TPU95", "PLA", "ABS", "PETG", "PET", "PC", "ASA", "PVA", "CoPA", "CoPE",
];

// A few product names don't carry their real material as a plain word in the title at all
// (marketing names like "PolyCast", "PolySmooth") or need a materials-list entry that would
// otherwise collide with generic tokens ("PA12") — explicit rather than guessed.
static OVERRIDES: LazyLock<HashMap<&'static str, (&'static str, Option<&'static str>)>> = LazyLock::new(|| {
    HashMap::from([
        ("PolyCast", ("PLA", Some("PolyCast"))),
        ("PolySmooth", ("PVB", Some("PolySmooth"))),
        ("PolyDissolve S1 (PVA)", ("PVA", Some("PolyDissolve S1"))),
        ("PolySupport for PA12", ("PA12", Some("PolySupport"))),
        ("PolySupport for PLA", ("PLA", Some("PolySupport"))),
        ("Panchroma Gradient Celestial", ("PLA", Some("Panchroma Gradient Celestial"))),
        ("Panchroma Gradient Crystal", ("PLA", Some("Panchroma Gradient Crystal"))),
        ("Panchroma Gradient Galaxy", ("PLA", Some("Panchroma Gradient Galaxy"))),
        ("Panchroma Gradient Neon", ("PLA", Some("Panchroma Gradient Neon"))),
        ("Panchroma Gradient Silk", ("PLA", Some("Panchroma Gradient Silk"))),
        ("Panchroma Gradient Starlight", ("PLA", Some("Panchroma Gradient Starlight"))),
    ])
});

static WHITESPACE_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s+").unwrap());

// Some titles use U+00A0 (non-breaking space) after the trademark symbol instead of a plain
// space — normalize both away before any matching.
fn normalize(title: &str) -> String {
    WHITESPACE_RE.replace_all(&title.replace('™', "").replace('\u{a0}', " "), " ").trim().to_string()
}

fn clean_up(s: &str) -> String {
    WHITESPACE_RE.replace_all(s, " ").trim_matches(|c| c == ' ' || c == '-').to_string()
}

fn split_material_variant(raw_title: &str) -> (String, Option<String>) {
    let title = normalize(raw_title);

    if let Some((m, v)) = OVERRIDES.iter().find(|(k, _)| k.eq_ignore_ascii_case(&title)).map(|(_, v)| *v) {
        return (m.to_string(), v.map(str::to_string));
    }
    if title.to_lowercase().starts_with("fiberon") {
        return (clean_up(&title["Fiberon".len()..]), None);
    }

    let brand = BRAND_LINES.iter().find(|b| title.to_lowercase().starts_with(&b.to_lowercase()));
    let rest = match brand {
        Some(b) => title[b.len()..].trim().to_string(),
        None => title.clone(),
    };

    let material = KNOWN_MATERIALS.iter().find(|m| word_boundary_contains(&rest, m));
    let Some(material) = material else {
        return (clean_up(&rest), brand.map(|b| clean_up(b)));
    };

    let remainder = clean_up(&word_boundary_replace(&rest, material, ""));
    let variant = clean_up(&format!("{} {}", brand.unwrap_or(&""), remainder));
    (material.to_string(), if variant.is_empty() { None } else { Some(variant) })
}

fn word_boundary_contains(haystack: &str, needle: &str) -> bool {
    Regex::new(&format!(r"(?i)\b{}\b", regex::escape(needle))).unwrap().is_match(haystack)
}

fn word_boundary_replace(haystack: &str, needle: &str, replacement: &str) -> String {
    Regex::new(&format!(r"(?i)\b{}\b", regex::escape(needle)))
        .unwrap()
        .replace_all(haystack, replacement)
        .into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_collection_keeps_only_filament_product_types() {
        let json = r#"
            {
              "products": [
                { "title": "PolyLite™ PLA", "product_type": "Polymaker Filament", "options": [{ "name": "Color", "values": ["Black", "White"] }] },
                { "title": "Blue Starter Pack", "product_type": "Bundle Packs", "options": [{ "name": "Color", "values": ["Mixed"] }] },
                { "title": "Creator Special Edition: Hedgehog", "product_type": "Creator Spools", "options": [{ "name": "Color", "values": ["Mixed"] }] }
              ]
            }
        "#;
        let products = parse_collection(json);
        assert_eq!(products.len(), 1);
        assert_eq!(products[0].title, "PolyLite\u{2122} PLA");
    }

    #[test]
    fn parse_collection_skips_products_where_color_option_is_not_actually_colors() {
        let json = r#"
            {
              "products": [
                {
                  "title": "PolyLite™ CosPLA",
                  "product_type": "Polymaker Filament",
                  "options": [{ "name": "Color", "values": ["Version A - Durability with extra sand-ability", "Version B - Sand-ability with extra durability"] }]
                }
              ]
            }
        "#;
        assert!(parse_collection(json).is_empty());
    }

    #[test]
    fn parse_collection_skips_refill_products_with_unattributable_colors() {
        let json = r#"
            {
              "products": [
                {
                  "title": "Panchroma™ PLA Refill",
                  "product_type": "Panchroma Filament",
                  "options": [{ "name": "Color", "values": ["Matte Black", "Silk Gold"] }]
                }
              ]
            }
        "#;
        assert!(parse_collection(json).is_empty());
    }

    #[test]
    fn split_material_variant_handles_branded_titles() {
        let cases = [
            ("PolyLite\u{2122} PLA", "PLA", Some("PolyLite")),
            ("PolyLite\u{2122} PLA Pro", "PLA", Some("PolyLite Pro")),
            ("Panchroma\u{2122} Matte PLA", "PLA", Some("Panchroma Matte")),
            ("Panchroma\u{2122} CoPE", "CoPE", Some("Panchroma")),
            ("PolyMax\u{2122} PC-FR", "PC-FR", Some("PolyMax")),
            ("Polymaker PC-ABS", "PC-ABS", Some("Polymaker")),
            ("Polymaker\u{2122} HT-PLA-GF", "HT-PLA-GF", Some("Polymaker")),
            ("PolyLite\u{2122} LW-PLA", "LW-PLA", Some("PolyLite")),
            ("Fiberon\u{2122} ASA-CF08", "ASA-CF08", None),
            ("Fiberon\u{2122} PETG-ESD", "PETG-ESD", None),
            ("PolyCast\u{2122}", "PLA", Some("PolyCast")),
            ("PolySmooth\u{2122}", "PVB", Some("PolySmooth")),
            ("PolyDissolve\u{2122} S1 (PVA)", "PVA", Some("PolyDissolve S1")),
            ("PolySupport\u{2122} for PA12", "PA12", Some("PolySupport")),
            ("Panchroma\u{2122} Gradient Galaxy", "PLA", Some("Panchroma Gradient Galaxy")),
        ];
        for (title, mat, var) in cases {
            let (m, v) = split_material_variant(title);
            assert_eq!(m, mat, "material for {title:?}");
            assert_eq!(v, var.map(str::to_string), "variant for {title:?}");
        }
    }
}
