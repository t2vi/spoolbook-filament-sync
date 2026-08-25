use crate::cloak_browser_client::CloakBrowserClient;
use crate::filament_sync_entry::FilamentSyncEntry;
use crate::source::FilamentSource;
use regex::Regex;
use serde::Deserialize;
use std::sync::LazyLock;

// Prusa's shop (prusa3d.com) is a Next.js + GraphQL storefront — unlike the Shopify sites this
// needs a direct GraphQL POST rather than a products.json endpoint. Reverse-engineered via the
// server's own validation error messages (introspection is disabled) against the public,
// unauthenticated /graphql/ endpoint.
const GRAPHQL_URL: &str = "https://www.prusa3d.com/graphql/";

const QUERY: &str = r#"query {
  category(uuid: "dbab0081-dadb-44d1-9499-19446794319c") {
    products(
      first: 300
      filter: {
        priceOptionInput: { currencyCode: "USD", vatCountryCode: "US" }
        brands: ["24824bd8-0ec7-4cc5-aea4-c3ac7c16e2d0"]
      }
    ) {
      edges { node { ... on Variant { name } } }
    }
  }
}"#;

pub struct PrusamentSource;

#[async_trait::async_trait]
impl FilamentSource for PrusamentSource {
    fn name(&self) -> &'static str {
        "prusament"
    }

    async fn fetch(&self, _cloak: Option<&CloakBrowserClient>) -> Result<Vec<FilamentSyncEntry>, String> {
        let json = fetch_collection().await?;
        let names = parse_collection(&json);

        Ok(names
            .into_iter()
            .map(|name| {
                let (material, variant, color) = parse_product_name(&name);
                FilamentSyncEntry::new("Prusament", &material, variant, &color)
            })
            .collect())
    }
}

async fn fetch_collection() -> Result<String, String> {
    let client = reqwest::Client::builder()
        .user_agent(
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0 Safari/537.36",
        )
        .build()
        .map_err(|e| e.to_string())?;
    let response = client
        .post(GRAPHQL_URL)
        .json(&serde_json::json!({ "query": QUERY }))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    response.error_for_status().map_err(|e| e.to_string())?.text().await.map_err(|e| e.to_string())
}

#[derive(Deserialize)]
struct GraphQlResponse {
    data: Option<GraphQlData>,
}
#[derive(Deserialize)]
struct GraphQlData {
    category: Option<CategoryData>,
}
#[derive(Deserialize)]
struct CategoryData {
    products: Option<ProductsData>,
}
#[derive(Deserialize)]
struct ProductsData {
    edges: Vec<ProductEdge>,
}
#[derive(Deserialize)]
struct ProductEdge {
    node: ProductNode,
}
#[derive(Deserialize)]
struct ProductNode {
    name: Option<String>,
}

// Unlike the Shopify-backed stores (Elegoo/SUNLU/Polymaker), each Prusament "product" node here
// already IS a single color — no separate per-product Color option list.
fn parse_collection(json: &str) -> Vec<String> {
    let data: GraphQlResponse = match serde_json::from_str(json) {
        Ok(d) => d,
        Err(_) => return Vec::new(),
    };

    data.data
        .and_then(|d| d.category)
        .and_then(|c| c.products)
        .map(|p| p.edges)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|e| e.node.name)
        // Samples (25g) and multipacks resell the same color as the regular-size listing.
        .filter(|n| !n.to_lowercase().contains("sample") && !n.to_lowercase().contains("multipack"))
        .collect()
}

// Longest/most-specific first: "PEI 1010" and "TPU 95A" are atomic grade names, not a material
// + a separate color-prefix number.
const KNOWN_MATERIALS: &[&str] = &["TPU 95A", "PEI 1010", "PA11", "PETG", "PLA", "ASA", "PC", "PEI", "PP", "PVB", "TPU"];
const VARIANT_QUALIFIERS: &[&str] = &["Carbon Fiber", "Glass Fiber", "Blend"];

static WEIGHT_SUFFIX_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\s*\(?\d+(\.\d+)?\s*(g|kg)\)?.*$").unwrap());

fn parse_product_name(raw_name: &str) -> (String, Option<String>, String) {
    let mut t = raw_name;
    if let Some(rest) = t.strip_prefix("Prusament ") {
        t = rest;
    }

    let premium = t.starts_with("Premium ");
    if premium {
        t = &t["Premium ".len()..];
    }

    let t = WEIGHT_SUFFIX_RE.replace(t, "").trim().to_string();

    let material = KNOWN_MATERIALS.iter().find(|m| t.starts_with(&format!("{m} ")) || t == **m);
    let Some(material) = material else {
        return (t, None, String::new());
    };

    let mut rest = t[material.len()..].trim().to_string();

    let variant_qualifier = VARIANT_QUALIFIERS.iter().find(|v| rest.starts_with(&format!("{v} ")) || rest == **v);
    if let Some(vq) = variant_qualifier {
        rest = rest[vq.len()..].trim().to_string();
    }

    let mut variant_parts: Vec<&str> = Vec::new();
    if premium {
        variant_parts.push("Premium");
    }
    if let Some(vq) = variant_qualifier {
        variant_parts.push(vq);
    }
    let variant = if variant_parts.is_empty() { None } else { Some(variant_parts.join(" ")) };

    (material.to_string(), variant, rest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_collection_skips_sample_and_multipack_and_non_variant_entries() {
        let json = r#"
            {
              "data": {
                "category": {
                  "products": {
                    "edges": [
                      { "node": { "name": "Prusament PLA Jet Black 1kg (NFC)" } },
                      { "node": { "name": "Prusament PLA Jet Black 25g sample" } },
                      { "node": { "name": "Prusament TPU 95A Jet Black 500g (NFC) – Multipack 10 pcs" } },
                      { "node": {} }
                    ]
                  }
                }
              }
            }
        "#;
        let names = parse_collection(json);
        assert_eq!(names, vec!["Prusament PLA Jet Black 1kg (NFC)".to_string()]);
    }

    #[test]
    fn parse_product_name_splits_material_variant_color() {
        let cases = [
            ("Prusament PETG Matte Black 1kg (NFC)", "PETG", None, "Matte Black"),
            ("Prusament PLA Blend Royal Blue 1kg (NFC)", "PLA", Some("Blend"), "Royal Blue"),
            ("Prusament Premium PLA Mystic Brown 1kg (NFC)", "PLA", Some("Premium"), "Mystic Brown"),
            ("Prusament PC Blend Carbon Fiber Black 2kg (NFC)", "PC", Some("Blend"), "Carbon Fiber Black"),
            ("Prusament PA11 Carbon Fiber Black 800g (NFC)", "PA11", Some("Carbon Fiber"), "Black"),
            ("Prusament PP Glass Fiber Natural 850g (NFC)", "PP", Some("Glass Fiber"), "Natural"),
            ("Prusament TPU 95A Jet Black 500g (NFC)", "TPU 95A", None, "Jet Black"),
            ("Prusament PEI 1010 Natural 500g (NFC)", "PEI 1010", None, "Natural"),
            ("Prusament PETG Tungsten 75% 1kg (NFC)", "PETG", None, "Tungsten 75%"),
            ("Prusament PETG Magnetite 40% Grey 1kg (NFC)", "PETG", None, "Magnetite 40% Grey"),
            ("Prusament PETG Tungsten 75% (100g)", "PETG", None, "Tungsten 75%"),
            ("Prusament PETG Anthracite Grey 900g Refill (NFC Compatible)", "PETG", None, "Anthracite Grey"),
            ("Prusament PLA Prusa Orange 1kg (Clearance)", "PLA", None, "Prusa Orange"),
        ];
        for (name, mat, var, color) in cases {
            let (m, v, c) = parse_product_name(name);
            assert_eq!(m, mat, "material for {name:?}");
            assert_eq!(v, var.map(str::to_string), "variant for {name:?}");
            assert_eq!(c, color, "color for {name:?}");
        }
    }
}
