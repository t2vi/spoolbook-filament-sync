use crate::cloak_browser_client::CloakBrowserClient;
use crate::filament_sync_entry::FilamentSyncEntry;
use crate::source::FilamentSource;
use regex::Regex;
use serde::Deserialize;
use std::sync::LazyLock;

// Fillamentum's own site (fillamentum.com) is WordPress/WooCommerce with no product-data API;
// the actual store lives on the Shopify-backed shop.fillamentum.com subdomain instead.
const BASE_URL: &str = "https://shop.fillamentum.com";

pub struct FillamentumSource;

#[async_trait::async_trait]
impl FilamentSource for FillamentumSource {
    fn name(&self) -> &'static str {
        "fillamentum"
    }

    async fn fetch(&self, _cloak: Option<&CloakBrowserClient>) -> Result<Vec<FilamentSyncEntry>, String> {
        let products = fetch_all_products().await?;

        Ok(products
            .into_iter()
            .flat_map(|product| {
                let (material, variant, colors) = parse_product(&product);
                colors
                    .into_iter()
                    .map(move |color| FilamentSyncEntry::new("Fillamentum", &material, variant.clone(), &color))
                    .collect::<Vec<_>>()
            })
            .collect())
    }
}

async fn fetch_all_products() -> Result<Vec<FillamentumProduct>, String> {
    let client = reqwest::Client::builder()
        .user_agent(
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0 Safari/537.36",
        )
        .build()
        .map_err(|e| e.to_string())?;

    let mut result = Vec::new();
    for page in 1..=10 {
        let response = client
            .get(format!("{BASE_URL}/products.json?limit=250&page={page}"))
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let json = response.error_for_status().map_err(|e| e.to_string())?.text().await.map_err(|e| e.to_string())?;

        let page_products = parse_collection(&json);
        if page_products.is_empty() {
            break;
        }
        result.extend(page_products);
    }

    Ok(result)
}

#[derive(Debug, Clone, PartialEq)]
struct FillamentumProduct {
    title: String,
    variant_titles: Vec<String>,
}

#[derive(Deserialize)]
struct ShopifyCollection {
    products: Vec<ShopifyProduct>,
}
#[derive(Deserialize)]
struct ShopifyProduct {
    title: String,
    product_type: String,
    variants: Option<Vec<ShopifyVariant>>,
}
#[derive(Deserialize)]
struct ShopifyVariant {
    title: String,
}

// "15 m Sample"/"6 m Sample"/"Swatch"/"Sampler" listings re-sell small cuts of colors that
// already exist as full-size products under the same material line — real colors, but duplicate
// ones, so skip the whole listing rather than double-counting.
static SAMPLE_LISTING_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)^\d+\s*m sample\b|^swatch\b|^sampler\b").unwrap());

fn parse_collection(json: &str) -> Vec<FillamentumProduct> {
    let data: ShopifyCollection = match serde_json::from_str(json) {
        Ok(d) => d,
        Err(_) => return Vec::new(),
    };

    data.products
        .into_iter()
        .filter(|p| p.product_type == "filament for 3D printing")
        .filter(|p| !SAMPLE_LISTING_RE.is_match(&p.title))
        // "+ LockPAd" bundles a spool-lock accessory onto colors already listed as their own
        // standalone products (e.g. "Nylon FX256 \"Sky Blue\"") — pure duplicate noise.
        .filter(|p| !p.title.to_lowercase().contains("lockpad"))
        .map(|p| FillamentumProduct {
            title: p.title,
            variant_titles: p.variants.unwrap_or_default().into_iter().map(|v| v.title).collect(),
        })
        .collect()
}

// Ordered longest-prefix-first: base product-line text (title up to the quoted color, or the
// whole title when there's no quote) maps to a canonical (Material, Variant) pair matching the
// app's existing material vocabulary (e.g. "Nylon" not "PA", "PP" not "Polypropylene", "rPETG"
// not "rePETG").
const MATERIAL_MAP: &[(&str, &str, Option<&str>)] = &[
    ("ABS Extrafill", "ABS", Some("Extrafill")),
    ("ASA CF10 Carbon", "ASA", Some("CF10 Carbon")),
    ("ASA Extrafill", "ASA", Some("Extrafill")),
    ("CPE CF112 Carbon", "CPE", Some("CF112 Carbon")),
    ("CPE HG100", "CPE", Some("HG100")),
    ("Flexfill TPE 90A", "TPE", Some("Flexfill 90A")),
    ("Flexfill TPU 92A", "TPU", Some("Flexfill 92A")),
    ("Flexfill TPU 98A", "TPU", Some("Flexfill 98A")),
    ("HIPS Extrafill", "HIPS", Some("Extrafill")),
    ("NonOilen\u{ae}", "NonOilen\u{ae}", None),
    ("Nylon AF80 Aramid", "Nylon", Some("AF80 Aramid")),
    ("Nylon CF15 Carbon", "Nylon", Some("CF15 Carbon")),
    ("Nylon FX256", "Nylon", Some("FX256")),
    ("0rCA\u{ae}", "Nylon", Some("0rCA\u{ae}")),
    ("PETG", "PETG", None),
    ("PLA Crystal Clear", "PLA", Some("Crystal Clear")),
    ("PLA Extrafill", "PLA", Some("Extrafill")),
    ("Polypropylene PP 2320", "PP", Some("2320")),
    ("Timberfill\u{ae}", "PLA", Some("Timberfill\u{ae}")),
    ("Vinyl 303", "Vinyl", Some("303")),
    ("rePETG Loopfill", "rPETG", Some("Loopfill")),
];

fn split_material_variant(base_line: &str) -> (String, Option<String>) {
    let mut sorted: Vec<&(&str, &str, Option<&str>)> = MATERIAL_MAP.iter().collect();
    sorted.sort_by_key(|(prefix, _, _)| std::cmp::Reverse(prefix.len()));

    for (prefix, material, variant) in sorted {
        if base_line.starts_with(prefix) {
            return (material.to_string(), variant.map(str::to_string));
        }
    }

    (base_line.to_string(), None)
}

static QUOTE_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new("\"([^\"]+)\"").unwrap());
static PIPE_SUFFIX_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s*\|.*$").unwrap());
static WEIGHT_OR_DIAMETER_TOKEN_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)^\d+(\.\d+)?\s*(mm|kg|g)$").unwrap());

fn parse_product(product: &FillamentumProduct) -> (String, Option<String>, Vec<String>) {
    if let Some(quote_match) = QUOTE_RE.captures(&product.title) {
        let quote_start = quote_match.get(0).unwrap().start();
        let base_line = product.title[..quote_start].trim_end_matches(['|', ' ']);
        let (material, variant) = split_material_variant(base_line);
        let color = quote_match[1].to_string();
        let colors = if color.eq_ignore_ascii_case("Custom Color") { vec![] } else { vec![color] };
        return (material, variant, colors);
    }

    let no_quote_base = PIPE_SUFFIX_RE.replace(&product.title, "").trim().to_string();
    let (material, variant) = split_material_variant(&no_quote_base);

    let mut extracted_colors: Vec<String> = product
        .variant_titles
        .iter()
        .filter_map(|t| t.split('/').map(str::trim).next_back())
        .filter(|last| {
            !last.is_empty()
                && !last.eq_ignore_ascii_case("Default Title")
                && !WEIGHT_OR_DIAMETER_TOKEN_RE.is_match(last)
        })
        .map(str::to_string)
        .collect();
    extracted_colors.sort();
    extracted_colors.dedup();

    let colors = if extracted_colors.is_empty() { vec!["Natural".to_string()] } else { extracted_colors };
    (material, variant, colors)
}

#[cfg(test)]
mod tests {
    use super::*;

    const LISTING_JSON: &str = r#"
        {
          "products": [
            {
              "title": "PLA Extrafill \"Natural\" | 1 KG | 1.75 mm",
              "product_type": "filament for 3D printing",
              "variants": [{ "title": "1.75 mm / 1 Kg" }]
            },
            {
              "title": "15 m Sample | 1.75 mm | PP 2320",
              "product_type": "filament for 3D printing",
              "variants": [{ "title": "1.75 mm / Natural" }, { "title": "1.75 mm / Black" }]
            },
            {
              "title": "Nylon FX256 + LockPAd",
              "product_type": "filament for 3D printing",
              "variants": [{ "title": "\"Natural\" 1.75 mm" }]
            },
            {
              "title": "PLA Extrafill Tool",
              "product_type": "Tools",
              "variants": [{ "title": "Default Title" }]
            }
          ]
        }
    "#;

    #[test]
    fn parse_collection_keeps_only_real_filament_products() {
        let products = parse_collection(LISTING_JSON);
        assert_eq!(products.len(), 1);
        assert_eq!(products[0].title, "PLA Extrafill \"Natural\" | 1 KG | 1.75 mm");
    }

    #[test]
    fn parse_product_quoted_color_returns_material_variant_color() {
        let cases: &[(&str, &str, Option<&str>, &[&str])] = &[
            ("PLA Extrafill \"Natural\" | 1 KG | 1.75 mm", "PLA", Some("Extrafill"), &["Natural"]),
            ("PLA Extrafill \"Traffic White\"", "PLA", Some("Extrafill"), &["Traffic White"]),
            ("PLA Extrafill \"Everybody's Magenta\" | 1 KG | 1.75 mm", "PLA", Some("Extrafill"), &["Everybody's Magenta"]),
            ("PLA Extrafill \"Witch Please!\"", "PLA", Some("Extrafill"), &["Witch Please!"]),
            ("NonOilen\u{ae} \"Ginger Shot\"", "NonOilen\u{ae}", None, &["Ginger Shot"]),
            ("Flexfill TPU 92A \"Luminous Green\"", "TPU", Some("Flexfill 92A"), &["Luminous Green"]),
            ("Flexfill TPE 90A \"Traffic Black\"", "TPE", Some("Flexfill 90A"), &["Traffic Black"]),
            ("ASA CF10 Carbon \"Natural\"", "ASA", Some("CF10 Carbon"), &["Natural"]),
            ("Nylon FX256 \"Sky Blue\"", "Nylon", Some("FX256"), &["Sky Blue"]),
            ("rePETG Loopfill \"Custom Color\"", "rPETG", Some("Loopfill"), &[]),
        ];
        for (title, mat, var, colors) in cases {
            let product = FillamentumProduct { title: title.to_string(), variant_titles: vec!["1.75 mm / 1 Kg".to_string()] };
            let (m, v, c) = parse_product(&product);
            assert_eq!(&m, mat, "material for {title:?}");
            assert_eq!(v.as_deref(), *var, "variant for {title:?}");
            assert_eq!(c, colors.to_vec(), "colors for {title:?}");
        }
    }

    #[test]
    fn parse_product_no_quoted_color_extracts_colors_from_variant_titles() {
        let product = FillamentumProduct {
            title: "Polypropylene PP 2320".to_string(),
            variant_titles: vec!["1.75 mm / Natural".to_string(), "1.75 mm / Black".to_string()],
        };
        let (material, variant, colors) = parse_product(&product);
        assert_eq!(material, "PP");
        assert_eq!(variant.as_deref(), Some("2320"));
        assert_eq!(colors, vec!["Black".to_string(), "Natural".to_string()]);
    }

    #[test]
    fn parse_product_no_quoted_color_and_no_color_variant_defaults_to_natural() {
        let product = FillamentumProduct {
            title: "Nylon AF80 Aramid".to_string(),
            variant_titles: vec!["1.75 mm".to_string(), "2.85 mm".to_string()],
        };
        let (material, variant, colors) = parse_product(&product);
        assert_eq!(material, "Nylon");
        assert_eq!(variant.as_deref(), Some("AF80 Aramid"));
        assert_eq!(colors, vec!["Natural".to_string()]);
    }

    #[test]
    fn parse_product_default_title_variant_defaults_to_natural() {
        let product = FillamentumProduct {
            title: "0rCA\u{ae} | Nylon PA6 + CF10 | 600 g | 1.75".to_string(),
            variant_titles: vec!["Default Title".to_string()],
        };
        let (material, variant, colors) = parse_product(&product);
        assert_eq!(material, "Nylon");
        assert_eq!(variant.as_deref(), Some("0rCA\u{ae}"));
        assert_eq!(colors, vec!["Natural".to_string()]);
    }

    #[test]
    fn parse_product_unknown_material_line_uses_title_as_material_with_null_variant() {
        let product = FillamentumProduct { title: "Mystery Filament \"Blue\"".to_string(), variant_titles: vec!["1.75 mm".to_string()] };
        let (material, variant, colors) = parse_product(&product);
        assert_eq!(material, "Mystery Filament");
        assert_eq!(variant, None);
        assert_eq!(colors, vec!["Blue".to_string()]);
    }
}
