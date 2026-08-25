use crate::cloak_browser_client::CloakBrowserClient;
use crate::filament_sync_entry::FilamentSyncEntry;
use crate::source::FilamentSource;
use regex::Regex;
use std::collections::HashSet;
use std::sync::LazyLock;
use std::time::Duration;

// Bambu's own store 429s after a burst of requests (observed ~15 rapid fetches). A fixed delay
// between requests plus a Retry-After-aware backoff keeps this well under that.
const BASE_URL: &str = "https://au.store.bambulab.com";
const REQUEST_DELAY: Duration = Duration::from_millis(1500);

pub struct BambuSource;

#[async_trait::async_trait]
impl FilamentSource for BambuSource {
    fn name(&self) -> &'static str {
        "bambu"
    }

    async fn fetch(&self, _cloak: Option<&CloakBrowserClient>) -> Result<Vec<FilamentSyncEntry>, String> {
        let client = build_client()?;

        println!("Fetching Bambu Lab filament listing...");
        let listing_html = fetch(&client, &format!("{BASE_URL}/collections/bambu-lab-3d-printer-filament")).await?;
        // Bundle/multi-pack SKUs re-sell existing PLA Basic colors under a packaging-descriptor
        // "variant" (e.g. "Basic Refill Pack 10 Rolls") rather than a real product variant —
        // skip them so they don't create near-duplicate catalog rows for colors already
        // captured.
        let skip_slugs: HashSet<&str> = HashSet::from(["pla-basic-refill-bundle", "pla-basic-refill-pack"]);
        let slugs: Vec<String> =
            extract_product_slugs(&listing_html).into_iter().filter(|s| !skip_slugs.contains(s.as_str())).collect();
        println!("Found {} products.", slugs.len());

        let mut result = Vec::new();
        for slug in slugs {
            println!("Fetching {slug}...");
            let product_html = match fetch(&client, &format!("{BASE_URL}/en/products/{slug}")).await {
                Ok(h) => h,
                Err(e) => {
                    eprintln!("Skipping {slug}: {e}");
                    continue;
                }
            };

            let page = parse_product_page(&product_html);
            let Some(name) = page.name else { continue };
            if page.colors.is_empty() {
                continue;
            }

            let (material, variant) = split_material_variant(&name);
            for color in page.colors {
                let color = strip_material_prefix(&color, &material);
                result.push(FilamentSyncEntry::new("Bambu Lab", &material, variant.clone(), &color));
            }
        }

        Ok(result)
    }
}

fn build_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent(
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0 Safari/537.36",
        )
        .build()
        .map_err(|e| e.to_string())
}

async fn fetch(client: &reqwest::Client, url: &str) -> Result<String, String> {
    loop {
        tokio::time::sleep(REQUEST_DELAY).await;
        let response = client.get(url).send().await.map_err(|e| e.to_string())?;

        if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            let retry_after = response
                .headers()
                .get(reqwest::header::RETRY_AFTER)
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<u64>().ok())
                .map(Duration::from_secs)
                .unwrap_or(Duration::from_secs(10));
            tokio::time::sleep(retry_after).await;
            continue;
        }

        return response.error_for_status().map_err(|e| e.to_string())?.text().await.map_err(|e| e.to_string());
    }
}

static PRODUCT_LINK_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"href="/en/products/([a-z0-9-]+)""#).unwrap());

fn extract_product_slugs(listing_html: &str) -> Vec<String> {
    let mut seen = Vec::new();
    for cap in PRODUCT_LINK_RE.captures_iter(listing_html) {
        let slug = cap[1].to_string();
        if !seen.contains(&slug) {
            seen.push(slug);
        }
    }
    seen
}

struct ProductPage {
    name: Option<String>,
    colors: Vec<String>,
}

static H1_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?s)<h1[^>]*>(.*?)</h1>").unwrap());
static COLOR_SWATCH_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"<li value="([^"(]+) \(\d+\)""#).unwrap());

fn parse_product_page(html: &str) -> ProductPage {
    let name = H1_RE.captures(html).map(|c| c[1].trim().to_string());

    let mut colors = Vec::new();
    for cap in COLOR_SWATCH_RE.captures_iter(html) {
        let color = cap[1].trim().to_string();
        if !colors.contains(&color) {
            colors.push(color);
        }
    }

    ProductPage { name, colors }
}

fn strip_material_prefix(color: &str, material: &str) -> String {
    let prefix = format!("{material} ");
    match color.strip_prefix(&prefix) {
        Some(rest) => rest.to_string(),
        None => color.to_string(),
    }
}

fn split_material_variant(product_name: &str) -> (String, Option<String>) {
    match product_name.find(' ') {
        None => (product_name.to_string(), None),
        Some(idx) => (product_name[..idx].to_string(), Some(product_name[idx + 1..].to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_product_slugs_returns_unique_sorted_slugs() {
        let html = r#"
            <a href="/en/products/pla-basic-filament">PLA Basic</a>
            <a href="/en/products/petg-hf">PETG HF</a>
            <a href="/en/products/pla-basic-filament">PLA Basic</a>
            <a href="/other/not-a-product">nope</a>
        "#;
        assert_eq!(extract_product_slugs(html), vec!["pla-basic-filament".to_string(), "petg-hf".to_string()]);
    }

    #[test]
    fn parse_product_page_extracts_name_and_deduped_colors() {
        let html = r#"
            <title>PLA Basic | Bambu Lab AU Store</title>
            <h1 class="ProductTitle">PLA Basic</h1>
            <li value="Bambu Green (10501)" class="swatch"><img src="a.png"/></li>
            <li value="Mistletoe Green (10502)" class="swatch"><img src="b.png"/></li>
            <li value="Bambu Green (10501)" class="swatch"><img src="a.png"/></li>
        "#;
        let page = parse_product_page(html);
        assert_eq!(page.name.as_deref(), Some("PLA Basic"));
        assert_eq!(page.colors, vec!["Bambu Green".to_string(), "Mistletoe Green".to_string()]);
    }

    #[test]
    fn parse_product_page_no_color_swatches_returns_empty_colors() {
        let html = "<h1>Bambu Reusable Spool</h1>";
        let page = parse_product_page(html);
        assert!(page.colors.is_empty());
    }

    #[test]
    fn strip_material_prefix_removes_leading_material_name_if_present() {
        assert_eq!(strip_material_prefix("ABS Orange", "ABS"), "Orange");
        assert_eq!(strip_material_prefix("ABS", "ABS"), "ABS");
        assert_eq!(strip_material_prefix("Black", "ABS"), "Black");
    }

    #[test]
    fn split_material_variant_splits_on_first_space() {
        let cases: &[(&str, &str, Option<&str>)] = &[
            ("PLA Basic", "PLA", Some("Basic")),
            ("PLA Basic Gradient", "PLA", Some("Basic Gradient")),
            ("PETG HF", "PETG", Some("HF")),
            ("ASA-CF", "ASA-CF", None),
            ("ABS", "ABS", None),
            ("TPU For AMS", "TPU", Some("For AMS")),
        ];
        for (name, mat, var) in cases {
            let (m, v) = split_material_variant(name);
            assert_eq!(&m, mat, "material for {name:?}");
            assert_eq!(v.as_deref(), *var, "variant for {name:?}");
        }
    }
}
