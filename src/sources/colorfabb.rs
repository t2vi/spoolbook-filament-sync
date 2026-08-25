use crate::cloak_browser_client::CloakBrowserClient;
use crate::filament_sync_entry::FilamentSyncEntry;
use crate::source::FilamentSource;
use regex::Regex;
use std::collections::HashSet;
use std::sync::LazyLock;

// colorFabb's Magento listing page is fixed at 12 products per page (query-string page-size
// overrides are silently ignored) with ~533 products total, so this paginates through the full
// "/filaments" category rather than a single bulk request like the Shopify-backed scrapers.
const BASE_URL: &str = "https://colorfabb.com";

pub struct ColorfabbSource;

#[async_trait::async_trait]
impl FilamentSource for ColorfabbSource {
    fn name(&self) -> &'static str {
        "colorfabb"
    }

    async fn fetch(&self, _cloak: Option<&CloakBrowserClient>) -> Result<Vec<FilamentSyncEntry>, String> {
        let titles = fetch_all_product_titles().await?;
        let real_titles = filter_real_products(titles);

        Ok(real_titles
            .into_iter()
            .map(|title| {
                let (material, variant, color) = parse_product_title(&title);
                FilamentSyncEntry::new("Colorfabb", &material, variant, &color)
            })
            .collect())
    }
}

async fn fetch_all_product_titles() -> Result<Vec<String>, String> {
    let client = reqwest::Client::builder()
        .user_agent(
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0 Safari/537.36",
        )
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;

    let mut titles = Vec::new();
    let mut seen = HashSet::new();

    for page in 1..=60 {
        if page > 1 {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }

        let response = client
            .get(format!("{BASE_URL}/filaments?p={page}"))
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let html = response.error_for_status().map_err(|e| e.to_string())?.text().await.map_err(|e| e.to_string())?;

        let page_titles = extract_product_titles(&html);
        let mut new_count = 0;
        for title in page_titles {
            if seen.insert(title.clone()) {
                titles.push(title);
                new_count += 1;
            }
        }

        println!("  page {page}: {} new, total {}", new_count, titles.len());
        if new_count == 0 && page > 1 {
            break;
        }
    }

    Ok(titles)
}

static PRODUCT_LINK_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"<a[^>]*class="product-item-link"[^>]*href="[^"]+"[^>]*>([^<]*)</a>"#).unwrap());

// colorFabb is Magento-based, not Shopify — each color is its own standalone product page
// (confirmed via a sample page's "alternative color" widget, which turned out to be a cross-sell
// link to sibling materials, not a variant selector), and the listing page's own link text
// already carries the full product title, so no per-product page fetch is needed.
fn extract_product_titles(listing_html: &str) -> Vec<String> {
    PRODUCT_LINK_RE.captures_iter(listing_html).map(|c| c[1].trim().to_string()).collect()
}

// "Luvocom"/"IGUS" are third-party industrial materials resold (not colorFabb-branded);
// "VALUE PACK" is a multi-color bundle, "PLAQUE SAMPLE" duplicates a real color as a swatch.
const EXCLUDE_PATTERNS: &[&str] = &["Luvocom", "IGUS", "VALUE PACK", "PLAQUE SAMPLE"];

fn filter_real_products(titles: Vec<String>) -> Vec<String> {
    titles.into_iter().filter(|t| !EXCLUDE_PATTERNS.iter().any(|p| t.contains(p))).collect()
}

// The "XXXfill" composite-material lines (metal/mineral/wood-filled PLA) are single-color — the
// fill name itself is the closest thing to a color description.
fn fill_variant(t: &str) -> Option<&'static str> {
    match t.to_uppercase().as_str() {
        "STEELFILL" => Some("steelFill"),
        "CORKFILL" => Some("corkFill"),
        "COPPERFILL" => Some("copperFill"),
        "BRONZEFILL" => Some("bronzeFill"),
        "WOODFILL" => Some("woodFill"),
        "GLOWFILL" => Some("glowFill"),
        _ => None,
    }
}

fn fill_color(variant: &str) -> &'static str {
    match variant {
        "steelFill" => "Steel",
        "corkFill" => "Cork",
        "copperFill" => "Copper",
        "bronzeFill" => "Bronze",
        "woodFill" => "Wood",
        "glowFill" => "Glow",
        _ => unreachable!(),
    }
}

// "Smokey Black"/"Milky White" are curated RAL-line favorites without an explicit RAL code in
// the title (confirmed via breadcrumb: PLA Filaments > RAL Favorites).
const RAL_FAVORITES: &[&str] = &["Smokey Black", "Milky White"];

// A few product codes carry no separable color word at all (confirmed via product pages) —
// "Natural" is the closest honest description available, not a guess.
fn code_only_material(t: &str) -> Option<&'static str> {
    match t.to_lowercase().as_str() {
        "ngen-cf10" => Some("nGen-CF10"),
        "xt-cf20" => Some("XT-CF20"),
        "pa neat" => Some("PA"),
        "pa-cf low warp" => Some("PA-CF"),
        _ => None,
    }
}

// Longest/most-specific first so e.g. "LW-PLA-HT" matches before "LW-PLA", "rPLA" before "PLA".
const KNOWN_MATERIALS: &[&str] = &[
    "LW-PLA-HT", "LW-PLA", "LW-ASA", "PLA-HP", "PLA/PHA", "allPHA", "rPETG", "rPLA",
    "PETG", "PLA", "ASA", "PA-CF", "PA", "TPU 95A", "TPU 85A", "TPU",
    "XT-CF20", "XT", "nGen-CF10", "nGen Flex", "nGen",
];

// Product-line qualifiers that appear alongside the material but aren't part of it — kept as
// Variant so e.g. "PLA Economy Black" and "PLA Black" aren't conflated.
const VARIANT_QUALIFIERS: &[&str] = &[
    "High Speed Pro", "Semi-Matte", "Semi Matte", "Economy", "Regrind",
    "Chameleon", "Vertigo", "Vibers", "Prosthetic", "Metal Detectable", "Varioshore",
];

static TRAILING_SIZE_SPEC_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s*\d+(\.\d+)?\s*/\s*\d+\s*$").unwrap());
static WHITESPACE_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s+").unwrap());
static RAL_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"RAL \d+").unwrap());

fn title_case(s: &str) -> String {
    s.split_whitespace()
        .map(|w| {
            let mut chars = w.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn word_boundary_find(haystack: &str, needle: &str) -> Option<(usize, usize)> {
    Regex::new(&format!(r"(?i)\b{}\b", regex::escape(needle))).unwrap().find(haystack).map(|m| (m.start(), m.end()))
}

fn word_boundary_replace(haystack: &str, needle: &str, replacement: &str) -> String {
    Regex::new(&format!(r"(?i)\b{}\b", regex::escape(needle))).unwrap().replace_all(haystack, replacement).into_owned()
}

fn parse_product_title(raw_title: &str) -> (String, Option<String>, String) {
    let t = raw_title.trim();

    if RAL_RE.is_match(t) || RAL_FAVORITES.contains(&t) {
        return ("PLA".to_string(), Some("RAL".to_string()), t.to_string());
    }

    if let Some(fv) = fill_variant(t) {
        return ("PLA".to_string(), Some(fv.to_string()), fill_color(fv).to_string());
    }

    if t.to_uppercase().starts_with("STONEFILL") {
        return ("PLA".to_string(), Some("stoneFill".to_string()), title_case(t["STONEFILL".len()..].trim()));
    }

    if t.to_uppercase().starts_with("VARIOSHORE PROSTHETIC") {
        return (
            "TPU".to_string(),
            Some("Varioshore Prosthetic".to_string()),
            title_case(t["VARIOSHORE PROSTHETIC".len()..].trim()),
        );
    }

    if let Some(code_material) = code_only_material(t) {
        return (code_material.to_string(), None, "Natural".to_string());
    }

    let t = t.replace('_', " ");
    let t = TRAILING_SIZE_SPEC_RE.replace(&t, "");
    let t = WHITESPACE_RE.replace_all(&t, " ").trim().to_string();

    let material = KNOWN_MATERIALS.iter().find(|m| word_boundary_find(&t, m).is_some());
    let Some(material) = material else {
        return (t.clone(), None, t);
    };

    let rest = word_boundary_replace(&t, material, "");
    let rest = rest.trim_matches(|c| c == ' ' || c == '-').to_string();

    let variant_qualifier = VARIANT_QUALIFIERS.iter().find(|v| word_boundary_find(&rest, v).is_some());
    let (rest, variant) = match variant_qualifier {
        Some(vq) => {
            let r = word_boundary_replace(&rest, vq, "");
            let r = r.trim_matches(|c| c == ' ' || c == '-').to_string();
            (r, Some(title_case(vq)))
        }
        None => (rest, None),
    };

    let rest = Regex::new(r"[\s-]+").unwrap().replace_all(&rest, " ").trim_matches(|c| c == ' ' || c == '-').to_string();
    (material.to_string(), variant, if rest.is_empty() { "Natural".to_string() } else { rest })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_real_products_excludes_third_party_rebrands_bundles_and_samples() {
        let titles = vec![
            "VARIOSHORE TPU BLACK".to_string(),
            "Luvocom 3F PET CF 9780 BK".to_string(),
            "IGUS IGLIDUR I150".to_string(),
            "LW-PLA VALUE PACK".to_string(),
            "Milky White  PLAQUE SAMPLE".to_string(),
        ];
        assert_eq!(filter_real_products(titles), vec!["VARIOSHORE TPU BLACK".to_string()]);
    }

    #[test]
    fn parse_product_title_splits_material_variant_color() {
        let cases: &[(&str, &str, Option<&str>, &str)] = &[
            ("Blue green RAL 6004", "PLA", Some("RAL"), "Blue green RAL 6004"),
            ("Smokey Black", "PLA", Some("RAL"), "Smokey Black"),
            ("Milky White", "PLA", Some("RAL"), "Milky White"),
            ("STEELFILL", "PLA", Some("steelFill"), "Steel"),
            ("BRONZEFILL", "PLA", Some("bronzeFill"), "Bronze"),
            ("STONEFILL MOSS GREEN", "PLA", Some("stoneFill"), "Moss Green"),
            ("VARIOSHORE PROSTHETIC PALE PINK", "TPU", Some("Varioshore Prosthetic"), "Pale Pink"),
            ("VARIOSHORE TPU BLACK", "TPU", Some("Varioshore"), "BLACK"),
            ("rPETG Burnt Amber", "rPETG", None, "Burnt Amber"),
            ("rPLA-Semi-matte-Monumental", "rPLA", Some("Semi-matte"), "Monumental"),
            ("PLA High Speed Pro Iron Grey", "PLA", Some("High Speed Pro"), "Iron Grey"),
            ("TPU 95A BLUE", "TPU 95A", None, "BLUE"),
            ("LW-PLA-HT DARK GRAY", "LW-PLA-HT", None, "DARK GRAY"),
            ("PLA/PHA VIOLET TRANSPARENT", "PLA/PHA", None, "VIOLET TRANSPARENT"),
            ("allPHA WHITE", "allPHA", None, "WHITE"),
            ("NGEN_FLEX DARK GRAY", "nGen Flex", None, "DARK GRAY"),
            ("PETG ECONOMY CLEAR", "PETG", Some("Economy"), "CLEAR"),
            ("PA Blue Metal Detectable", "PA", Some("Metal Detectable"), "Blue"),
            ("nGen-CF10", "nGen-CF10", None, "Natural"),
            ("XT-CF20", "XT-CF20", None, "Natural"),
            ("PA Neat", "PA", None, "Natural"),
            ("VARIOSHORE TPU GREEN 1.75 / 4200", "TPU", Some("Varioshore"), "GREEN"),
        ];
        for (title, mat, var, color) in cases {
            let (m, v, c) = parse_product_title(title);
            assert_eq!(&m, mat, "material for {title:?}");
            assert_eq!(v.as_deref(), *var, "variant for {title:?}");
            assert_eq!(&c, color, "color for {title:?}");
        }
    }
}
