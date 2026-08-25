use crate::cloak_browser_client::CloakBrowserClient;
use crate::filament_sync_entry::FilamentSyncEntry;
use crate::source::FilamentSource;
use regex::Regex;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::LazyLock;

// Single request only (Shopify's products.json returns the whole collection at once), same
// shape as Elegoo — SUNLU's actual storefront is store.sunlu.com, not www.sunlu.com (which is
// a separate Nuxt marketing site with no plain-HTTP-scrapeable product data).
const BASE_URL: &str = "https://store.sunlu.com";

pub struct SunluSource;

#[async_trait::async_trait]
impl FilamentSource for SunluSource {
    fn name(&self) -> &'static str {
        "sunlu"
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
                    .map(move |color| FilamentSyncEntry::new("SUNLU", &material, variant.clone(), &color))
            })
            .collect())
    }
}

async fn fetch_collection() -> Result<String, String> {
    let url = format!("{BASE_URL}/collections/3d-printer-filament/products.json?limit=250");
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
struct SunluProduct {
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
    options: Option<Vec<ShopifyOption>>,
}

#[derive(Deserialize)]
struct ShopifyOption {
    name: String,
    values: Vec<String>,
}

static MOQ_PREFIX_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)^\[MOQ[^\]]*\]\s*").unwrap());
static BULK_SIZE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)large spool|clearance|special offers|\b[3-9]\d*\s*kg\b").unwrap());

const BUNDLE_DETECT_MATERIALS: &[&str] = &["PLA", "PETG", "ABS", "ASA", "TPU", "PC"];

// "Large Spool"/explicit 3-9kg mentions are real bulk repackagings of the same colors as the
// 1kg listing. Rust's regex crate has no lookbehind, so the ".9KG" exclusion (normal
// single-roll fill weight, must not false-positive on the "9kg" inside "0.9kg") is done as a
// manual preceding-char check instead of C#'s `(?<!\.)`.
fn is_bulk_size(title: &str) -> bool {
    BULK_SIZE_RE.find_iter(title).any(|m| {
        let matched = m.as_str();
        if !matched.chars().next().is_some_and(|c| c.is_ascii_digit()) {
            return true; // "large spool" / "clearance" / "special offers"
        }
        !title[..m.start()].ends_with('.')
    })
}

fn parse_collection(json: &str) -> Vec<SunluProduct> {
    let data: ShopifyCollection = match serde_json::from_str(json) {
        Ok(d) => d,
        Err(_) => return Vec::new(),
    };

    data.products
        .into_iter()
        .filter_map(|p| {
            let title = MOQ_PREFIX_RE.replace(&p.title, "").into_owned();
            if is_bulk_size(&title) {
                return None;
            }

            let distinct_materials = BUNDLE_DETECT_MATERIALS
                .iter()
                .filter(|m| word_boundary_contains(&title, m))
                .count();
            if distinct_materials >= 2 {
                return None;
            }

            let colors: Vec<String> = p
                .options
                .unwrap_or_default()
                .into_iter()
                .find(|o| o.name == "Color")?
                .values
                .into_iter()
                .map(|c| clean_color_name(&c))
                .collect();
            if colors.is_empty() {
                return None;
            }

            Some(SunluProduct { title, colors })
        })
        .collect()
}

fn word_boundary_contains(haystack: &str, needle: &str) -> bool {
    Regex::new(&format!(r"(?i)\b{}\b", regex::escape(needle))).unwrap().is_match(haystack)
}

// SUNLU's own Color option values often repeat the product name (e.g. "PLA Galaxy | Starlit
// Flow", "PETG White", "Cherry Wood 1KG") instead of just the color — strip the redundant part
// rather than seed it verbatim.
const COLOR_PREFIXES_TO_STRIP: &[&str] = &[
    "High Speed Matte PETG", "Anti-string PLA", "High Speed PLA",
    "PLA+ 2.0", "PLA+", "PLA", "PETG", "Matte", "Silk", "Twinkling",
];

fn clean_color_name(color: &str) -> String {
    let mut color = match color.rfind(['|', '/']) {
        Some(idx) => color[idx + 1..].trim().to_string(),
        None => color.to_string(),
    };

    for prefix in COLOR_PREFIXES_TO_STRIP {
        let with_space = format!("{prefix} ");
        if color.len() >= with_space.len() && color[..with_space.len()].eq_ignore_ascii_case(&with_space) {
            color = color[with_space.len()..].trim().to_string();
            break;
        }
    }

    if color.len() >= 4 && color[color.len() - 4..].eq_ignore_ascii_case(" 1KG") {
        color = color[..color.len() - 4].trim().to_string();
    }

    color
}

// SUNLU's titles are inconsistent (parenthetical aliases like "PLA+(PLA Plus)", mixed word
// order like "Glow in The Dark (Luminous) PLA" vs "Matte PLA") — a generic token-match split
// (like Elegoo's) produced garbage on ~40% of these, so this is an explicit lookup instead.
static MATERIAL_VARIANT_BY_TITLE: LazyLock<HashMap<&'static str, (&'static str, Option<&'static str>)>> =
    LazyLock::new(|| {
        HashMap::from([
            ("SUNLU PLA+(PLA Plus) 3D Printer Filament 1KG", ("PLA", Some("Plus"))),
            ("SILK 3D Printer Filament 1KG", ("PLA", Some("Silk"))),
            ("ABS 3D Printer Filament 0.9KG/1KG", ("ABS", None)),
            ("PETG 3D Printer Filament 1KG", ("PETG", None)),
            ("Matte PLA 3D Printer Filament 1KG", ("PLA", Some("Matte"))),
            ("ASA 3D Printer Filament 1KG", ("ASA", None)),
            (
                "E ABS(Easy ABS) 3D Printer Filament 1KG(p.s.: For New Refill Spool is 0.9kg)",
                ("ABS", Some("Easy")),
            ),
            ("Glow in The Dark (Luminous) PLA 3D Printer Filament 1KG", ("PLA", Some("Glow in the Dark"))),
            ("TPU-SILK(SILK-Textured TPU) 3D Printer Filament 1KG", ("TPU", Some("Silk"))),
            ("Twinkling 3D Printer PLA Filament 1KG", ("PLA", Some("Twinkling"))),
            ("High Speed PLA(HS_PLA) 3D Printer Filament 1KG", ("PLA", Some("High Speed"))),
            ("TPU 3D Printer Filament 1KG, TPU 90A/TPU 95A", ("TPU", None)),
            ("PLA+ 2.0, Upgraded PLA+(PLA Plus), 3D Printer Filament 1KG", ("PLA", Some("Plus 2.0"))),
            ("APLA (Anti-string PLA) 3D Printer Filament 1KG", ("PLA", Some("Anti-string"))),
            ("PETG Rainbow Filament 3D Printer Filament 1KG", ("PETG", Some("Rainbow"))),
            (
                "Optimized Wood PLA 3D Printer Filament 1KG, Optimized and Upgraded Wood Texture",
                ("PLA", Some("Wood")),
            ),
            ("PETG Glow in The Dark (Luminous) 3D Printer Filament 1KG", ("PETG", Some("Glow in the Dark"))),
            ("High Speed Matte PETG 3D Printer Filament 1KG", ("PETG", Some("High Speed Matte"))),
            ("PETG-CF(PETG Carbon Fiber) 3D Printer Filament 1KG", ("PETG-CF", None)),
            ("High Speed Matte PLA 3D Printer Filament 1KG", ("PLA", Some("High Speed Matte"))),
            ("High Speed PLA+(PLA Plus), HS_PLA+ 3D Printer Filament 1KG", ("PLA", Some("High Speed Plus"))),
            (
                "High Speed PLA+ 2.0(HSPLA Plus 2.0), High Speed 3D Printer Filament 1KG",
                ("PLA", Some("High Speed Plus 2.0")),
            ),
            (
                "SUNLU PLA Galaxy 1KG, Color-Shifting PLA Esthenic Filament, Sparkling Ultrafine Pearlescent Powder",
                ("PLA", Some("Galaxy")),
            ),
            (
                "SUNLU Matte PLA Dual-Color 3D Printer Esthenic Filament 1KG, Seamless Two-Tone Shifts & Soft Matte Finish",
                ("PLA", Some("Matte Dual-Color")),
            ),
        ])
    });

fn split_material_variant(title: &str) -> (String, Option<String>) {
    MATERIAL_VARIANT_BY_TITLE
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(title))
        .map(|(_, (m, v))| (m.to_string(), v.map(str::to_string)))
        .unwrap_or_else(|| (title.to_string(), None))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_collection_keeps_moq_listings_strips_moq_prefix_from_title() {
        let json = r#"
            {
              "products": [
                {
                  "title": "[MOQ: 6KG] SILK 3D Printer Filament 1KG",
                  "options": [{ "name": "Color", "values": ["Black", "White"] }]
                }
              ]
            }
        "#;
        let products = parse_collection(json);
        assert_eq!(products.len(), 1);
        assert_eq!(products[0].title, "SILK 3D Printer Filament 1KG");
    }

    #[test]
    fn parse_collection_skips_bulk_size_repackagings() {
        let json = r#"
            {
              "products": [
                { "title": "PLA Large Spool 3D Printer Filament 5KG", "options": [{ "name": "Color", "values": ["Black"] }] },
                { "title": "PLA 3KG Large Spool 3D Printer Filament 3KG", "options": [{ "name": "Color", "values": ["Black"] }] },
                { "title": "ABS 3D Printer Filament 0.9KG/1KG", "options": [{ "name": "Color", "values": ["Black"] }] }
              ]
            }
        "#;
        let products = parse_collection(json);
        assert_eq!(products.len(), 1);
        assert_eq!(products[0].title, "ABS 3D Printer Filament 0.9KG/1KG");
    }

    #[test]
    fn parse_collection_skips_multi_material_bundles_with_unattributable_colors() {
        let json = r#"
            {
              "products": [
                {
                  "title": "[Australia Only] SUNLU Basic Filament Collection – PLA, PLA+, PETG",
                  "options": [{ "name": "Color", "values": ["Black", "White", "Grey"] }]
                },
                {
                  "title": "TPU-SILK(SILK-Textured TPU) 3D Printer Filament 1KG",
                  "options": [{ "name": "Color", "values": ["Black", "Cream White"] }]
                }
              ]
            }
        "#;
        let products = parse_collection(json);
        assert_eq!(products.len(), 1);
        assert_eq!(products[0].title, "TPU-SILK(SILK-Textured TPU) 3D Printer Filament 1KG");
    }

    #[test]
    fn split_material_variant_looks_up_known_title() {
        let cases = [
            ("SUNLU PLA+(PLA Plus) 3D Printer Filament 1KG", "PLA", Some("Plus")),
            ("SILK 3D Printer Filament 1KG", "PLA", Some("Silk")),
            ("ABS 3D Printer Filament 0.9KG/1KG", "ABS", None),
            ("TPU-SILK(SILK-Textured TPU) 3D Printer Filament 1KG", "TPU", Some("Silk")),
            ("PETG-CF(PETG Carbon Fiber) 3D Printer Filament 1KG", "PETG-CF", None),
            (
                "High Speed PLA+ 2.0(HSPLA Plus 2.0), High Speed 3D Printer Filament 1KG",
                "PLA",
                Some("High Speed Plus 2.0"),
            ),
        ];
        for (title, mat, var) in cases {
            let (m, v) = split_material_variant(title);
            assert_eq!(m, mat, "material for {title:?}");
            assert_eq!(v, var.map(str::to_string), "variant for {title:?}");
        }
    }

    #[test]
    fn parse_collection_strips_redundant_material_prefix_from_color_names() {
        let cases = [
            ("1KG ABS | Black", "Black"),
            ("High Speed Matte PETG | Black", "Black"),
            ("PLA Galaxy | Starlit Flow", "Starlit Flow"),
            ("Anti-string PLA / Black", "Black"),
            ("PETG White", "White"),
            ("PLA+ Black", "Black"),
            ("PLA+ 2.0 | Black", "Black"),
            ("Matte White", "White"),
            ("Silk Black", "Black"),
            ("Twinkling Blue", "Blue"),
            ("Cherry Wood 1KG", "Cherry Wood"),
            ("Red Filament (Glow Red)", "Red Filament (Glow Red)"),
            ("Red+Yellow", "Red+Yellow"),
        ];
        for (raw, expected) in cases {
            let json = format!(
                r#"{{ "products": [{{ "title": "Test Product 3D Printer Filament 1KG", "options": [{{ "name": "Color", "values": ["{raw}"] }}] }}] }}"#
            );
            let products = parse_collection(&json);
            assert_eq!(products[0].colors, vec![expected.to_string()], "color for {raw:?}");
        }
    }

    #[test]
    fn split_material_variant_unknown_title_falls_back_to_raw_title_as_material() {
        let (material, variant) = split_material_variant("Some Brand New SUNLU Product");
        assert_eq!(material, "Some Brand New SUNLU Product");
        assert_eq!(variant, None);
    }
}
