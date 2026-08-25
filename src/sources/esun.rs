use crate::cloak_browser_client::CloakBrowserClient;
use crate::filament_sync_entry::FilamentSyncEntry;
use crate::source::FilamentSource;
use regex::Regex;
use std::sync::LazyLock;
use std::time::Duration;

// Mirrors BambuStoreClient's rate-limit-friendly shape; eSUN's own limits are unconfirmed so we
// apply the same conservative delay/backoff rather than hammering the site to find out.
const BASE_URL: &str = "https://esun3dstore.com";
const REQUEST_DELAY: Duration = Duration::from_millis(1500);

pub struct EsunSource;

#[async_trait::async_trait]
impl FilamentSource for EsunSource {
    fn name(&self) -> &'static str {
        "esun"
    }

    async fn fetch(&self, _cloak: Option<&CloakBrowserClient>) -> Result<Vec<FilamentSyncEntry>, String> {
        let client = build_client()?;

        // Multi-roll/bundle SKUs (e.g. "PLA-Basic 4 Rolls", "3KG Spool") encode pack contents
        // as pseudo-"colors" in their Color picker (e.g. "Black+Cold White" for a 2-roll mixed
        // pack) — not real single-spool colors, so skip the whole slug rather than the colors
        // it emits.
        let bundle_slug_re = Regex::new(r"(?i)rolls|spool|combo|bundle|kit|resin|box|vacuum|swatch").unwrap();

        let mut slugs: Vec<String> = Vec::new();
        for page in 1..=20 {
            println!("Fetching eSUN listing page {page}...");
            let listing_html = fetch_listing(&client, page).await?;
            let page_slugs = extract_product_slugs(&listing_html);
            if page_slugs.is_empty() {
                break;
            }
            for slug in page_slugs {
                if !slugs.contains(&slug) && !bundle_slug_re.is_match(&slug) {
                    slugs.push(slug);
                }
            }
        }
        println!("Found {} products.", slugs.len());

        let mut result = Vec::new();
        for slug in slugs {
            println!("Fetching {slug}...");
            let product_html = match fetch_product(&client, &slug).await {
                Ok(h) => h,
                Err(e) => {
                    eprintln!("Skipping {slug}: {e}");
                    continue;
                }
            };

            // ParseProductPage returns a null Name for non-filament products (e.g. dry boxes,
            // accessories) that share the same collection listing as the actual filaments.
            let page = parse_product_page(&product_html);
            let Some(name) = page.name else { continue };
            if page.colors.is_empty() {
                continue;
            }

            let (material, variant) = split_material_variant(&name);
            // "eSpool+" is a reusable-spool-holder upsell eSUN embeds directly in the Color
            // picker on some product pages (e.g. pla-refilament) — not a real color.
            for color in page.colors.into_iter().filter(|c| c != "eSpool+") {
                result.push(FilamentSyncEntry::new("eSUN", &material, variant.clone(), &color));
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

async fn fetch_listing(client: &reqwest::Client, page: u32) -> Result<String, String> {
    let url = if page <= 1 {
        format!("{BASE_URL}/collections/3d-filament")
    } else {
        format!("{BASE_URL}/collections/3d-filament?page={page}")
    };
    fetch(client, &url).await
}

async fn fetch_product(client: &reqwest::Client, slug: &str) -> Result<String, String> {
    fetch(client, &format!("{BASE_URL}/products/{slug}")).await
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

static PRODUCT_LINK_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"href="/products/([a-z0-9-]+)""#).unwrap());

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

static FILAMENT_TITLE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?s)<h1[^>]*>eSUN\s+(.+?)\s+[\d.]+mm 3D Filament.*?</h1>").unwrap());
// The wrapping <li>'s data-type is "color" for hex swatches or "picture" for photo thumbnails,
// depending on the product — name="Color" is the constant across both.
static COLOR_BLOCK_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?s)name="Color"[^>]*data-type="(?:color|picture)".*?>(.*?)</li>"#).unwrap());
// Color options render either as a plain hex swatch (data-is-picture="0") or a product photo
// thumbnail (data-is-picture="1") — both are real colors, only the former carries hex.
static COLOR_SWATCH_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"data-title="([^"]+)"\s+data-is-picture="[01]""#).unwrap());

fn parse_product_page(html: &str) -> ProductPage {
    let name = FILAMENT_TITLE_RE.captures(html).map(|c| c[1].trim().to_string());

    // Scope color extraction to the Color <li> block specifically — the same data-title/
    // data-is-picture markup is reused elsewhere on the page for bundle/addon pickers (e.g.
    // multi-roll packs, "eSpool+" upsell), which aren't real colors.
    let color_block = COLOR_BLOCK_RE.captures(html).map(|c| c[1].to_string()).unwrap_or_default();

    let mut colors = Vec::new();
    for cap in COLOR_SWATCH_RE.captures_iter(&color_block) {
        let color = cap[1].trim().to_string();
        if !colors.contains(&color) {
            colors.push(color);
        }
    }

    ProductPage { name, colors }
}

fn split_material_variant(name: &str) -> (String, Option<String>) {
    if let Some(plus_index) = name.find('+') {
        let material = name[..plus_index].to_string();
        let rest = name[plus_index + 1..].trim();
        let variant = if rest.is_empty() { "Plus".to_string() } else { format!("Plus {rest}") };
        return (material, Some(variant));
    }

    if let Some(dash_index) = name.find('-') {
        return (name[..dash_index].to_string(), Some(name[dash_index + 1..].to_string()));
    }

    (name.to_string(), None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_product_slugs_returns_unique_slugs_in_order() {
        let html = r#"
            <a href="/products/epla">PLA-Basic</a>
            <a href="/products/pla-pro">PLA+</a>
            <a href="/products/epla">PLA-Basic</a>
            <a href="/collections/resin">not a product</a>
        "#;
        assert_eq!(extract_product_slugs(html), vec!["epla".to_string(), "pla-pro".to_string()]);
    }

    #[test]
    fn parse_product_page_extracts_name_and_deduped_colors() {
        let html = r##"
            <h1 itemprop="name" class="detail_name themes_products_title" >eSUN PLA-Basic 1.75mm 3D Filament 1KG</h1>
            <li class="attr_show attr_show_list" name="Color" data-position="1" data-type="color" data-picture="0">
                <div class="attr_box">
                    <div value="Black" data="{&quot;Price&quot;:0}" class="btn_attr" data-title="Black" data-is-picture="0">
                        <span class="attr_color" style="background-color:#393d47"></span>
                    </div>
                    <div value="Cold White" data="{&quot;Price&quot;:0}" class="btn_attr" data-title="Cold White" data-is-picture="0">
                        <span class="attr_color" style="background-color:#f8f8ff"></span>
                    </div>
                    <div value="Black" data="{&quot;Price&quot;:0}" class="btn_attr" data-title="Black" data-is-picture="0">
                        <span class="attr_color" style="background-color:#393d47"></span>
                    </div>
                </div>
            </li>
        "##;
        let page = parse_product_page(html);
        assert_eq!(page.name.as_deref(), Some("PLA-Basic"));
        assert_eq!(page.colors, vec!["Black".to_string(), "Cold White".to_string()]);
    }

    #[test]
    fn parse_product_page_ignores_non_color_attr_options() {
        let html = r##"
            <h1 itemprop="name" class="detail_name themes_products_title" >eSUN PETG 1.75mm 3D Filament 1KG</h1>
            <li class="attr_show attr_show_list" name="Ship From" data-type="text">
                <div class="attr_box">
                    <div value="3" class="btn_attr" data-title="US">US</div>
                </div>
            </li>
            <li class="attr_show attr_show_list" name="Color" data-position="1" data-type="color" data-picture="0">
                <div class="attr_box">
                    <div value="Black" class="btn_attr" data-title="Black" data-is-picture="0">
                        <span class="attr_color" style="background-color:#000000"></span>
                    </div>
                </div>
            </li>
        "##;
        let page = parse_product_page(html);
        assert_eq!(page.colors, vec!["Black".to_string()]);
    }

    #[test]
    fn parse_product_page_picture_based_swatches_have_no_hex_span_but_still_count_as_colors() {
        let html = r##"
            <h1 itemprop="name" class="detail_name themes_products_title" >eSUN PETG 1.75mm 3D Filament 1KG</h1>
            <li class="attr_show attr_show_list" name="Color" data-position="1" data-type="picture" data-picture="0">
                <div class="attr_box">
                    <div value="Solid Black" data="{&quot;Price&quot;:0}" class="btn_attr" data-title="Solid Black" data-is-picture="1">
                        <img src="thumb.jpg"/>
                    </div>
                    <div value="Solid White" data="{&quot;Price&quot;:0}" class="btn_attr" data-title="Solid White" data-is-picture="1">
                        <img src="thumb2.jpg"/>
                    </div>
                </div>
            </li>
        "##;
        let page = parse_product_page(html);
        assert_eq!(page.colors, vec!["Solid Black".to_string(), "Solid White".to_string()]);
    }

    #[test]
    fn parse_product_page_ignores_bundle_and_addon_pickers_outside_color_block() {
        let html = r##"
            <h1 itemprop="name" class="detail_name themes_products_title" >eSUN PLA-Basic 1.75mm 3D Filament 1KG</h1>
            <li class="attr_show attr_show_list" name="Color" data-position="1" data-type="color" data-picture="0">
                <div class="attr_box">
                    <div value="Black" class="btn_attr" data-title="Black" data-is-picture="0">
                        <span class="attr_color" style="background-color:#000000"></span>
                    </div>
                </div>
            </li>
            <li class="attr_show attr_show_list" name="Bundle" data-type="bundle">
                <div class="attr_box">
                    <div value="Black 4rolls" class="btn_attr" data-title="Black 4rolls" data-is-picture="1">
                        <img src="bundle.jpg"/>
                    </div>
                    <div value="Classic Bundle" class="btn_attr" data-title="Classic Bundle" data-is-picture="1">
                        <img src="bundle2.jpg"/>
                    </div>
                </div>
            </li>
            <div class="ajax_you_make_also_like" attrid="1">
                <div value="eSpool+" class="btn_attr" data-title="eSpool+" data-is-picture="1">
                    <img src="addon.jpg"/>
                </div>
            </div>
        "##;
        let page = parse_product_page(html);
        assert_eq!(page.colors, vec!["Black".to_string()]);
    }

    #[test]
    fn parse_product_page_non_filament_product_returns_null_name() {
        let html = r#"<h1 itemprop="name" class="detail_name themes_products_title" >eSUN Dry Box Lite</h1>"#;
        let page = parse_product_page(html);
        assert_eq!(page.name, None);
    }

    #[test]
    fn split_material_variant_splits_on_plus_or_dash() {
        let cases: &[(&str, &str, Option<&str>)] = &[
            ("PLA-Basic", "PLA", Some("Basic")),
            ("PLA+", "PLA", Some("Plus")),
            ("PETG", "PETG", None),
            ("ABS+", "ABS", Some("Plus")),
            ("TPU-95A", "TPU", Some("95A")),
            ("PLA-Silk Magic", "PLA", Some("Silk Magic")),
            ("PLA+ Refilament", "PLA", Some("Plus Refilament")),
        ];
        for (name, mat, var) in cases {
            let (m, v) = split_material_variant(name);
            assert_eq!(&m, mat, "material for {name:?}");
            assert_eq!(v.as_deref(), *var, "variant for {name:?}");
        }
    }
}
