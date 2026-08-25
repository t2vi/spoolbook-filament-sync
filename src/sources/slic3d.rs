use crate::cloak_browser_client::CloakBrowserClient;
use crate::filament_sync_entry::FilamentSyncEntry;
use crate::source::FilamentSource;
use regex::Regex;
use std::sync::LazyLock;

const URL: &str = "https://www.jaycar.com.au/brands/slic3d";

// Jaycar's own house brand (no independent manufacturer storefront exists — see spoolbook's
// docs/adr/0012 "reversing abandon if blocked" addendum) — jaycar.com.au is Slic3D's own
// store in the same sense hatchbox3d.com is Hatchbox's. Blocked by DataDome, so fetched via
// CloakBrowser. Small brand (17 SKUs, single page, no pagination) with no products.json-style
// API — the Next.js page ships product data purely client-side, but each product image's alt
// text is a clean, human-readable title ("Slic3D PETG filament Black 1.75mm 1kg"), so no need
// to reverse-engineer the API.
pub struct Slic3DSource;

#[async_trait::async_trait]
impl FilamentSource for Slic3DSource {
    fn name(&self) -> &'static str {
        "slic3d"
    }

    async fn fetch(&self, cloak: Option<&CloakBrowserClient>) -> Result<Vec<FilamentSyncEntry>, String> {
        let cloak = cloak.ok_or("slic3d requires a CloakBrowser client")?;
        let html = cloak.fetch_page_html(URL, 45_000).await?;
        let titles = parse_listing_page(&html);

        Ok(titles
            .into_iter()
            .map(|title| {
                let (material, variant, color) = parse_product_title(&title);
                FilamentSyncEntry::new("Slic3D", &material, variant, &color)
            })
            .collect())
    }
}

static ALT_TEXT_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"alt="(Slic3D[^"]*)""#).unwrap());

fn parse_listing_page(html: &str) -> Vec<String> {
    let mut seen = Vec::new();
    for cap in ALT_TEXT_RE.captures_iter(html) {
        let title = cap[1].to_string();
        if !seen.contains(&title) {
            seen.push(title);
        }
    }
    seen
}

static FILAMENT_WORD_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)\bfilament\b").unwrap());
// Handles both "1.75mm 1kg" and the real "Yellow1.75mm 1kg" (no space before size).
static SIZE_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)\d+(\.\d+)?\s*mm\s*\d+\s*kg").unwrap());
static SPOOL_LESS_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)\bSpool-Less\b").unwrap());

const MATERIALS: &[&str] = &["PETG", "PLA"];

fn parse_product_title(alt_text: &str) -> (String, Option<String>, String) {
    let mut t = alt_text.replace("Slic3D", "").trim().to_string();
    t = FILAMENT_WORD_RE.replace_all(&t, "").into_owned();
    t = SIZE_RE.replace_all(&t, "").into_owned();

    let mut variant = None;
    if SPOOL_LESS_RE.is_match(&t) {
        variant = Some("Spool-Less".to_string());
        t = SPOOL_LESS_RE.replace_all(&t, "").into_owned();
    }

    let mut material = "Unknown".to_string();
    for &m in MATERIALS {
        let re = Regex::new(&format!(r"(?i)\b{m}\b")).unwrap();
        if re.is_match(&t) {
            material = m.to_string();
            t = re.replace_all(&t, "").into_owned();
            break;
        }
    }

    let color = Regex::new(r"\s+").unwrap().replace_all(t.trim(), " ").trim().to_string();
    (material, variant, color)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_listing_page_extracts_alt_text_from_product_images() {
        let html = r#"
            <img alt="Slic3D PETG filament Black 1.75mm 1kg" src="a.jpg">
            <img alt="Slic3D PLA Spool-Less White 1.75mm 1kg filament" src="b.jpg">
            <img alt="Not a product" src="c.jpg">
        "#;

        let titles = parse_listing_page(html);

        assert_eq!(
            titles,
            vec![
                "Slic3D PETG filament Black 1.75mm 1kg".to_string(),
                "Slic3D PLA Spool-Less White 1.75mm 1kg filament".to_string(),
            ]
        );
    }

    #[test]
    fn parse_product_title_extracts_material_variant_color() {
        let cases = [
            ("Slic3D PETG filament Black 1.75mm 1kg", "PETG", None, "Black"),
            ("Slic3D PETG filament Yellow1.75mm 1kg", "PETG", None, "Yellow"),
            ("Slic3D PLA Spool-Less Apple Green 1.75mm 1kg filament", "PLA", Some("Spool-Less"), "Apple Green"),
            ("Slic3D PLA Spool-Less Cocoa Brown 1.75mm 1kg filament", "PLA", Some("Spool-Less"), "Cocoa Brown"),
        ];

        for (alt_text, expected_material, expected_variant, expected_color) in cases {
            let (material, variant, color) = parse_product_title(alt_text);
            assert_eq!(material, expected_material, "material for {alt_text:?}");
            assert_eq!(variant, expected_variant.map(str::to_string), "variant for {alt_text:?}");
            assert_eq!(color, expected_color, "color for {alt_text:?}");
        }
    }
}
