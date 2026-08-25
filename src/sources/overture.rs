use crate::cloak_browser_client::CloakBrowserClient;
use crate::filament_sync_entry::FilamentSyncEntry;
use crate::source::FilamentSource;
use serde::Deserialize;

// Single request only (Shopify's products.json returns the whole catalog at once — Overture's
// storefront only sells filament, so no collection scoping is needed). overture3d.com's theme
// pages intermittently 503 (Shopify's own generic outage page, not a bot wall), but
// products.json itself has been reliable.
const BASE_URL: &str = "https://www.overture3d.com";

pub struct OvertureSource;

#[async_trait::async_trait]
impl FilamentSource for OvertureSource {
    fn name(&self) -> &'static str {
        "overture"
    }

    async fn fetch(&self, _cloak: Option<&CloakBrowserClient>) -> Result<Vec<FilamentSyncEntry>, String> {
        let json = fetch_collection().await?;
        let products = parse_collection(&json);

        Ok(products
            .into_iter()
            .flat_map(|product| {
                let (material, variant) = split_material_variant(&product.product_type);
                product
                    .colors
                    .into_iter()
                    .map(move |color| FilamentSyncEntry::new("Overture", &material, variant.clone(), &color))
            })
            .collect())
    }
}

async fn fetch_collection() -> Result<String, String> {
    let url = format!("{BASE_URL}/products.json?limit=250");
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
struct OvertureProduct {
    product_type: String,
    colors: Vec<String>,
}

#[derive(Deserialize)]
struct ShopifyCollection {
    products: Vec<ShopifyProduct>,
}

#[derive(Deserialize)]
struct ShopifyProduct {
    product_type: String,
    options: Option<Vec<ShopifyOption>>,
}

#[derive(Deserialize)]
struct ShopifyOption {
    name: String,
    values: Vec<String>,
}

fn parse_collection(json: &str) -> Vec<OvertureProduct> {
    let data: ShopifyCollection = match serde_json::from_str(json) {
        Ok(d) => d,
        Err(_) => return Vec::new(),
    };

    data.products
        .into_iter()
        .filter(|p| p.product_type.to_lowercase().starts_with("3d printer filament"))
        .filter_map(|p| {
            let colors: Vec<String> = p
                .options
                .unwrap_or_default()
                .into_iter()
                .find(|o| o.name == "Color")?
                .values
                .into_iter()
                .map(|c| normalize_color(&c))
                .collect();
            if colors.is_empty() {
                return None;
            }
            Some(OvertureProduct { product_type: p.product_type, colors })
        })
        .collect()
}

// Dual/gradient color names use a single hyphen ("Black-White") where every other scraper in
// this codebase uses "+" ("Black+White") — normalize so these get the same multi-color swatch
// treatment. Fancy single-word names never contain exactly one hyphen, so this is safe.
fn normalize_color(color: &str) -> String {
    if color.matches('-').count() == 1 {
        color.replacen('-', "+", 1)
    } else {
        color.to_string()
    }
}

// Overture's own product_type taxonomy already encodes Material/Variant hierarchically (e.g.
// "3D Printer Filament > PLA > MATTE PLA") — no title parsing needed.
fn split_material_variant(product_type: &str) -> (String, Option<String>) {
    let segments: Vec<&str> = product_type.split('>').map(str::trim).collect();
    let material = segments[1].to_string();
    let variant_raw = {
        let replaced = case_insensitive_replace(segments[2], &material, "");
        replaced.trim_matches(|c| c == ' ' || c == '+').to_string()
    };

    if variant_raw.is_empty() {
        return (material, None);
    }

    let variant = variant_raw.split_whitespace().map(title_case_word).collect::<Vec<_>>().join(" ");
    (material, Some(variant))
}

fn case_insensitive_replace(haystack: &str, needle: &str, replacement: &str) -> String {
    let lower_haystack = haystack.to_lowercase();
    let lower_needle = needle.to_lowercase();
    match lower_haystack.find(&lower_needle) {
        Some(idx) => format!("{}{}{}", &haystack[..idx], replacement, &haystack[idx + needle.len()..]),
        None => haystack.to_string(),
    }
}

fn title_case_word(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_collection_extracts_colors_and_normalizes_hyphenated_dual_color_names() {
        let json = r#"
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
        "#;
        let products = parse_collection(json);
        assert_eq!(products.len(), 1);
        assert_eq!(products[0].colors, vec!["Black+White", "Silk Tiger Eye"]);
    }

    #[test]
    fn split_material_variant_splits_hierarchical_product_type() {
        let cases = [
            ("3D Printer Filament > PLA > PLA", "PLA", None),
            ("3D Printer Filament > PLA > MATTE PLA", "PLA", Some("Matte")),
            ("3D Printer Filament > PLA > PLA PROFESSIONAL", "PLA", Some("Professional")),
            ("3D Printer Filament > PLA+> PLA+", "PLA+", None),
            ("3D Printer Filament > PLA > SUPER PLA+", "PLA", Some("Super")),
            ("3D Printer Filament > TPU > HIGH SPEED TPU", "TPU", Some("High Speed")),
            ("3D Printer Filament > Nylon > EASY NYLON", "Nylon", Some("Easy")),
            ("3D Printer Filament > PC > PC PROFESSIONAL", "PC", Some("Professional")),
        ];
        for (pt, mat, var) in cases {
            let (m, v) = split_material_variant(pt);
            assert_eq!(m, mat, "material for {pt:?}");
            assert_eq!(v, var.map(str::to_string), "variant for {pt:?}");
        }
    }
}
