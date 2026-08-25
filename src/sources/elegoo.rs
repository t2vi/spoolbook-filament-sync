use crate::cloak_browser_client::CloakBrowserClient;
use crate::filament_sync_entry::FilamentSyncEntry;
use crate::source::FilamentSource;
use regex::Regex;
use serde::Deserialize;
use std::sync::LazyLock;

const BASE_URL: &str = "https://www.elegoo.com";

// Elegoo's own store is a stock Shopify theme, so unlike Bambu/eSUN this reads the platform's
// public products.json endpoint directly instead of regexing HTML. Single request only (Shopify
// returns the whole collection at once) — no rate-limit delay loop needed.
pub struct ElegooSource;

#[async_trait::async_trait]
impl FilamentSource for ElegooSource {
    fn name(&self) -> &'static str {
        "elegoo"
    }

    async fn fetch(
        &self,
        _cloak: Option<&CloakBrowserClient>,
    ) -> Result<Vec<FilamentSyncEntry>, String> {
        let json = fetch_collection().await?;
        let products = parse_collection(&json);

        Ok(products
            .into_iter()
            .flat_map(|product| {
                let (material, variant) = split_material_variant(&product.title);
                product
                    .colors
                    .into_iter()
                    .map(move |color| {
                        FilamentSyncEntry::new("Elegoo", &material, variant.clone(), &color)
                    })
            })
            .collect())
    }
}

async fn fetch_collection() -> Result<String, String> {
    let url = format!("{BASE_URL}/collections/filaments/products.json?limit=250");
    let client = reqwest::Client::builder()
        .user_agent(
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0 Safari/537.36",
        )
        .build()
        .map_err(|e| e.to_string())?;
    let response = client.get(url).send().await.map_err(|e| e.to_string())?;
    response
        .error_for_status()
        .map_err(|e| e.to_string())?
        .text()
        .await
        .map_err(|e| e.to_string())
}

#[derive(Debug, PartialEq)]
struct ElegooProduct {
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

// Single-roll products (the ones we want) never mention a weight in the title (e.g. "PLA
// Plus", "ASA"). Bulk repackagings do ("PLA Filament 1.75mm Black 10KG", "Mini 250 g
// Filament Bundle") — same colors as the single roll, just resold in bulk, so skip them
// rather than create duplicate/garbage-variant catalog entries.
static BULK_WEIGHT_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\d+\s*(kg|g)\b").unwrap());

const KNOWN_MATERIALS: &[&str] = &["PLA-CF", "PLA", "PETG", "ASA", "TPU", "PC"];

fn parse_collection(json: &str) -> Vec<ElegooProduct> {
    let data: ShopifyCollection = match serde_json::from_str(json) {
        Ok(d) => d,
        Err(_) => return Vec::new(),
    };

    data.products
        .into_iter()
        .filter(|p| p.product_type == "3D Filaments")
        .filter(|p| !BULK_WEIGHT_RE.is_match(&p.title))
        .filter_map(|p| {
            // Some products (e.g. "PLA Matte") mix real colors with bulk-pack pseudo-options
            // ("2kg option1", "4KG-OPTION1") directly inside the same Color list.
            let colors: Vec<String> = p
                .options
                .unwrap_or_default()
                .into_iter()
                .find(|o| o.name == "Color")?
                .values
                .into_iter()
                .filter(|c| !BULK_WEIGHT_RE.is_match(c))
                .collect();

            if colors.is_empty() {
                return None;
            }

            Some(ElegooProduct { title: p.title, colors })
        })
        .collect()
}

// Matches a known material as a whole word (not substring) so e.g. "PLA-CF" and "PLA"
// never collide; whatever prefix/suffix words remain (e.g. "Rapid" + "Plus") become the Variant.
fn split_material_variant(title: &str) -> (String, Option<String>) {
    let words: Vec<&str> = title.split(' ').collect();
    let material_index = words
        .iter()
        .position(|w| KNOWN_MATERIALS.iter().any(|m| m.eq_ignore_ascii_case(w)));

    match material_index {
        None => (title.to_string(), None),
        Some(idx) => {
            let material = words[idx].to_string();
            let rest: Vec<&str> = words
                .iter()
                .enumerate()
                .filter(|(i, _)| *i != idx)
                .map(|(_, w)| *w)
                .collect();
            let variant = if rest.is_empty() { None } else { Some(rest.join(" ")) };
            (material, variant)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_collection_extracts_title_and_colors_skips_bulk_and_non_filament_products() {
        let json = r#"
            {
              "products": [
                {
                  "title": "PLA Plus",
                  "product_type": "3D Filaments",
                  "options": [{ "name": "Color", "position": 1, "values": ["Black", "White"] }]
                },
                {
                  "title": "PLA Filament 1.75mm Black 10KG",
                  "product_type": "3D Filaments",
                  "options": [{ "name": "Color", "position": 1, "values": ["Black"] }]
                },
                {
                  "title": "Mini 250 g Filament Bundle",
                  "product_type": "3D Filaments",
                  "options": [{ "name": "Color", "position": 1, "values": ["Mixed"] }]
                },
                {
                  "title": "3D Stainless Steel Funnel",
                  "product_type": "Accessories",
                  "options": [{ "name": "Title", "position": 1, "values": ["Default Title"] }]
                }
              ]
            }
        "#;

        let products = parse_collection(json);

        assert_eq!(products.len(), 1);
        assert_eq!(products[0].title, "PLA Plus");
        assert_eq!(products[0].colors, vec!["Black", "White"]);
    }

    #[test]
    fn parse_collection_filters_bulk_pack_pseudo_colors_within_a_product() {
        // Some products (e.g. "PLA Matte") mix real colors with bulk-pack pseudo-options
        // directly inside the same Color list, rather than as a separate bulk product.
        let json = r#"
            {
              "products": [
                {
                  "title": "PLA Matte",
                  "product_type": "3D Filaments",
                  "options": [{ "name": "Color", "position": 1, "values": ["Black", "2kg option1", "4KG-OPTION1"] }]
                }
              ]
            }
        "#;

        let products = parse_collection(json);

        assert_eq!(products.len(), 1);
        assert_eq!(products[0].colors, vec!["Black"]);
    }

    #[test]
    fn split_material_variant_matches_known_material_token() {
        let cases = [
            ("PLA Plus", "PLA", Some("Plus")),
            ("Rapid PLA Plus", "PLA", Some("Rapid Plus")),
            ("PLA Silk", "PLA", Some("Silk")),
            ("ASA", "ASA", None),
            ("Rapid PETG", "PETG", Some("Rapid")),
            ("TPU 95A", "TPU", Some("95A")),
            ("PLA-CF", "PLA-CF", None),
            ("PC", "PC", None),
        ];

        for (title, expected_material, expected_variant) in cases {
            let (material, variant) = split_material_variant(title);
            assert_eq!(material, expected_material, "material for {title:?}");
            assert_eq!(
                variant,
                expected_variant.map(str::to_string),
                "variant for {title:?}"
            );
        }
    }
}
