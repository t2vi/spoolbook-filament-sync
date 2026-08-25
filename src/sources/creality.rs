use crate::cloak_browser_client::CloakBrowserClient;
use crate::filament_sync_entry::FilamentSyncEntry;
use crate::source::FilamentSource;
use regex::Regex;
use serde::Deserialize;
use std::sync::LazyLock;

// Single request only (Shopify's products.json returns the whole collection at once) —
// www.creality.com/store.creality.com run a custom headless storefront with no server-rendered
// product data and no standard products.json endpoint, but us.store.creality.com is a plain
// Shopify theme.
const BASE_URL: &str = "https://us.store.creality.com";

pub struct CrealitySource;

#[async_trait::async_trait]
impl FilamentSource for CrealitySource {
    fn name(&self) -> &'static str {
        "creality"
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
                    .map(move |color| FilamentSyncEntry::new("Creality", &material, variant.clone(), &color))
            })
            .collect())
    }
}

async fn fetch_collection() -> Result<String, String> {
    let url = format!("{BASE_URL}/collections/materials/products.json?limit=250");
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
struct CrealityProduct {
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

// "White*2+Black*2" and "2-Pack"/"3-Pack(Grey)" are multi-roll pack pseudo-options baked into
// the same product's Color list, not real colors.
static PACK_MULTIPLIER_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)\*\d+|\d+-Pack").unwrap());

// A real two-tone blend ("Golden-silver") is exactly one Capitalized word, a hyphen, then one
// lowercase word. Fancy gradient names ("Wild Blossom-Long") have a multi-word or non-color
// side and must not be touched.
static TWO_TONE_COLOR_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[A-Z][a-z]+-[a-z]+$").unwrap());

fn normalize_color(color: &str) -> String {
    if TWO_TONE_COLOR_RE.is_match(color) {
        color.replacen('-', "+", 1)
    } else {
        color.to_string()
    }
}

fn parse_collection(json: &str) -> Vec<CrealityProduct> {
    let data: ShopifyCollection = match serde_json::from_str(json) {
        Ok(d) => d,
        Err(_) => return Vec::new(),
    };

    data.products
        .into_iter()
        // Resin shares the same store/collection shape but isn't a filament.
        .filter(|p| !p.title.to_lowercase().contains("resin"))
        .filter_map(|p| {
            let colors: Vec<String> = p
                .options
                .unwrap_or_default()
                .into_iter()
                .find(|o| o.name == "Color")?
                .values
                .into_iter()
                .filter(|c| !PACK_MULTIPLIER_RE.is_match(c))
                .map(|c| normalize_color(&c))
                .collect();
            if colors.is_empty() {
                return None;
            }
            Some(CrealityProduct { title: p.title, colors })
        })
        .collect()
}

const KNOWN_MATERIALS: &[&str] = &["PETG-CF", "PLA-CF", "PLA", "PETG", "ABS", "TPU", "ASA", "PC"];

static WEIGHT_OR_DIAMETER_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)\b\d+(\.\d+)?\s*(kg|mm)\b").unwrap());
static TRAILING_MARKETING_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\s*\d*d?\s*Print(ing|er) Filament.*$").unwrap());
static WHITESPACE_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s+").unwrap());

// Creality's product_type field is unreliable (mostly blank) — Material/Variant come from the
// title instead, after stripping weight/diameter tokens and the trailing "3D Printing/Printer
// Filament ..." marketing phrase.
fn split_material_variant(raw_title: &str) -> (String, Option<String>) {
    let t = TRAILING_MARKETING_RE.replace(raw_title, "");
    let t = WEIGHT_OR_DIAMETER_RE.replace_all(&t, "");
    let t = WHITESPACE_RE.replace_all(&t, " ").trim().to_string();

    let material = KNOWN_MATERIALS.iter().find(|m| word_boundary_contains(&t, m));
    let Some(material) = material else {
        return (t, None);
    };

    let variant = word_boundary_replace(&t, material, "");
    let variant = WHITESPACE_RE.replace_all(&variant, " ").trim_matches(|c| c == ' ' || c == '-').to_string();
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
    fn parse_collection_excludes_resin_and_bundles_and_pack_multiplier_pseudo_colors() {
        let json = r#"
            {
              "products": [
                {
                  "title": "Hyper Series PLA 3D Printing Filament 1kg",
                  "options": [{ "name": "Color", "values": ["White", "Black", "White*2+Black*2", "2-Pack", "3-Pack(Grey)"] }]
                },
                { "title": "Fast Resin UV Curable Resin 1KG", "options": [{ "name": "Color", "values": ["Grey", "Clear"] }] },
                { "title": "8KG Hyper PLA 6 Color Pack 3D Printing Filament", "options": [{ "name": "Title", "values": ["Default Title"] }] }
              ]
            }
        "#;
        let products = parse_collection(json);
        assert_eq!(products.len(), 1);
        assert_eq!(products[0].title, "Hyper Series PLA 3D Printing Filament 1kg");
        assert_eq!(products[0].colors, vec!["White", "Black"]);
    }

    #[test]
    fn parse_collection_normalizes_simple_two_tone_hyphenated_colors_but_not_fancy_names() {
        let json = r#"
            {
              "products": [
                {
                  "title": "CR-Silk 1.75mm PLA 3D Printing Filament 1kg",
                  "options": [{ "name": "Color", "values": ["Golden-silver", "Wild Blossom-Long"] }]
                }
              ]
            }
        "#;
        let products = parse_collection(json);
        assert_eq!(products[0].colors, vec!["Golden+silver", "Wild Blossom-Long"]);
    }

    #[test]
    fn split_material_variant_strips_noise_and_known_material_token() {
        let cases = [
            ("Hyper Series PLA 3D Printing Filament 1kg", "PLA", Some("Hyper Series")),
            ("Hyper PLA RFID 3D Printing Filament 1kg", "PLA", Some("Hyper RFID")),
            ("Hyper Series PLA Carbon Fibre 3D Printing Filament 1kg", "PLA", Some("Hyper Series Carbon Fibre")),
            ("CR-Silk 1.75mm PLA 3D Printing Filament 1kg", "PLA", Some("CR-Silk")),
            ("HP-TPU 3D Printer Filament 1.75mm 1kg", "TPU", Some("HP")),
            ("Hyper PETG-CF RFID 3D Printing Filament 1kg", "PETG-CF", Some("Hyper RFID")),
            ("CR PETG 3D Printing Filament 4kg", "PETG", Some("CR")),
        ];
        for (title, mat, var) in cases {
            let (m, v) = split_material_variant(title);
            assert_eq!(m, mat, "material for {title:?}");
            assert_eq!(v, var.map(str::to_string), "variant for {title:?}");
        }
    }
}
