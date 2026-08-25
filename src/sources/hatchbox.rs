use crate::cloak_browser_client::CloakBrowserClient;
use crate::filament_sync_entry::FilamentSyncEntry;
use crate::source::FilamentSource;
use regex::Regex;
use serde::Deserialize;
use std::sync::LazyLock;

const BASE_URL: &str = "https://hatchbox3d.com";

// hatchbox3d.com blocks plain reqwest requests (403 fingerprint challenge) — fetched via
// CloakBrowser instead (see spoolbook's docs/adr/0012 "reversing abandon if blocked" addendum).
pub struct HatchboxSource;

#[async_trait::async_trait]
impl FilamentSource for HatchboxSource {
    fn name(&self) -> &'static str {
        "hatchbox"
    }

    async fn fetch(&self, cloak: Option<&CloakBrowserClient>) -> Result<Vec<FilamentSyncEntry>, String> {
        let cloak = cloak.ok_or("hatchbox requires a CloakBrowser client")?;
        let titles = fetch_all_product_titles(cloak).await?;
        let real_titles = filter_real_products(titles.iter().map(String::as_str));

        Ok(real_titles
            .into_iter()
            .map(|title| {
                let (material, variant, color) = parse_product_title(title);
                FilamentSyncEntry::new(&material, &material, variant, &color)
            })
            .map(|mut e| {
                e.brand = "Hatchbox".to_string();
                e
            })
            .collect())
    }
}

async fn fetch_all_product_titles(cloak: &CloakBrowserClient) -> Result<Vec<String>, String> {
    let mut titles = Vec::new();
    for page in 1..=10 {
        let url = format!("{BASE_URL}/products.json?limit=250&page={page}");
        let html = cloak.fetch_page_html(&url, 60_000).await?;
        let page_titles = parse_collection(&html);
        if page_titles.is_empty() {
            break;
        }
        titles.extend(page_titles);
    }
    Ok(titles)
}

// Products come from products.json via CloakBrowser — Chromium wraps raw JSON responses in a
// <pre> tag when navigated to directly, and HTML-escapes it.
static PRE_TAG_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?s)<pre>(.*)</pre>").unwrap());

#[derive(Deserialize)]
struct ShopifyCollection {
    products: Vec<ShopifyProduct>,
}

#[derive(Deserialize)]
struct ShopifyProduct {
    title: String,
}

fn parse_collection(html_wrapped_json: &str) -> Vec<String> {
    let Some(captures) = PRE_TAG_RE.captures(html_wrapped_json) else {
        return Vec::new();
    };
    let json = decode_html_entities(&captures[1]);
    let data: ShopifyCollection = match serde_json::from_str(&json) {
        Ok(d) => d,
        Err(_) => return Vec::new(),
    };
    data.products.into_iter().map(|p| p.title.trim().to_string()).collect()
}

// Chromium HTML-escapes the JSON it wraps in <pre> — only the standard entities that ever show
// up in JSON text (quotes, angle brackets, ampersand) need handling.
fn decode_html_entities(s: &str) -> String {
    s.replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}

// Resin, gift cards, and merch don't contain "FILAMENT" in their title, so that check alone
// excludes them; 3D-pen filament and sample packs are a different product category (not
// spools); "Exclusive Release" is a one-off whose title carries no parseable color/material
// (its actual color-change behavior is only described in body_html).
const EXCLUDE_PATTERNS: &[&str] = &["3D PEN", "SAMPLE PACK", "CLEANING"];
const EXCLUDE_EXACT: &str = "Exclusive Release - Temp Change Filament";

fn filter_real_products<'a>(titles: impl Iterator<Item = &'a str>) -> Vec<&'a str> {
    titles
        .filter(|t| t.to_uppercase().contains("FILAMENT"))
        .filter(|t| *t != EXCLUDE_EXACT)
        .filter(|t| !EXCLUDE_PATTERNS.iter().any(|p| t.to_uppercase().contains(p)))
        .collect()
}

const MATERIALS: &[&str] = &["PLA", "ABS", "PETG", "TPU", "PA", "PC"];

// Product-line descriptors that sit next to color/material rather than being a color
// themselves — stripped from both ends of the remaining text (iteratively, since some
// products stack more than one, e.g. "Stone Gray Matte"), same algorithm as
// ProtopastaStoreParser (arbitrary, artisan-style naming needs the same iterative approach).
const QUALIFIERS: &[&str] = &[
    "Metallic Finish", "Temperature Color Changing", "UV Color Changing",
    "Glow In The Dark", "Carbon Fiber", "Paint Free", "Performance",
    "Transparent", "Silk", "Stone", "Wood", "Sparkle", "Matte", "Rapid",
    "Max V2", "Pro+",
];

fn strip_qualifiers(mut text: String) -> (String, Vec<&'static str>) {
    let mut matched = Vec::new();
    loop {
        let mut changed = false;
        for &q in QUALIFIERS {
            let qu = q.to_uppercase();
            if text == qu {
                text.clear();
                matched.push(q);
                changed = true;
                break;
            }
            if let Some(stripped) = text.strip_suffix(&format!(" {qu}")) {
                text = stripped.to_string();
                matched.push(q);
                changed = true;
                break;
            }
            if let Some(stripped) = text.strip_prefix(&format!("{qu} ")) {
                text = stripped.to_string();
                matched.push(q);
                changed = true;
                break;
            }
        }
        if !changed {
            break;
        }
    }
    (text.trim().to_string(), matched)
}

fn to_title_case(s: &str) -> String {
    s.to_lowercase()
        .split(' ')
        .map(|w| {
            let mut chars = w.chars();
            match chars.next() {
                Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

// Diameter/weight/packaging notes always come after "FILAMENT" in the title (e.g. "- 1.75MM,
// 1KG SPOOL", "REFILL & RELOADABLE SPOOL", "(SHORE 95A)") — irrelevant to Material/Variant/
// Color, and stripping at "FILAMENT" sidesteps needing to handle the inconsistent hyphen vs.
// en-dash the store uses before the size info.
fn parse_product_title(raw_title: &str) -> (String, Option<String>, String) {
    static FILAMENT_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new("(?i)FILAMENT").unwrap());
    static WHITESPACE_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s+").unwrap());

    let before_filament = FILAMENT_RE
        .split(raw_title.trim())
        .next()
        .unwrap_or(raw_title.trim())
        .trim()
        .to_string();

    for &material in MATERIALS {
        let m_re = Regex::new(&format!(r"(?i)\b{material}\b")).unwrap();
        if let Some(m) = m_re.find(&before_filament) {
            let remainder = format!(
                "{} {}",
                &before_filament[..m.start()],
                &before_filament[m.end()..]
            );
            let remainder = WHITESPACE_RE.replace_all(remainder.trim(), " ").into_owned();
            let (color_text, mut matched_qualifiers) = strip_qualifiers(remainder);

            matched_qualifiers.reverse();
            let variant = if matched_qualifiers.is_empty() {
                None
            } else {
                Some(matched_qualifiers.join(" "))
            };
            let color = if color_text.is_empty() { "Natural".to_string() } else { to_title_case(&color_text) };
            let material_name = if material == "PA" { "Nylon".to_string() } else { material.to_string() };
            return (material_name, variant, color);
        }
    }

    (before_filament.clone(), None, before_filament)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_real_products_excludes_non_filament_and_one_offs() {
        let titles = vec![
            "BLACK PLA FILAMENT - 1.75MM, 1KG SPOOL",
            "Yellow 8K 3D Printer Resin PRO - 405nm, 1000ml Bottle",
            "ABS 3D PEN FILAMENT SAMPLE PACK",
            "HATCHBOX T Shirt for Men",
            "Cleaning Filament - 1.75MM (.44 lbs)",
            "Exclusive Release - Temp Change Filament",
            "E-Gift Cards",
        ];

        let result = filter_real_products(titles.into_iter());

        assert_eq!(result, vec!["BLACK PLA FILAMENT - 1.75MM, 1KG SPOOL"]);
    }

    #[test]
    fn parse_product_title_extracts_material_variant_color() {
        let cases = [
            ("GOLD PETG FILAMENT - 1.75MM, 1KG SPOOL", "PETG", None, "Gold"),
            ("BLACK PLA FILAMENT - 1.75MM, 1KG SPOOL", "PLA", None, "Black"),
            ("BLACK PLA FILAMENT REFILL & RELOADABLE SPOOL - 1.75MM, 1KG SPOOL", "PLA", None, "Black"),
            ("BABY BLUE TPU FILAMENT - 1.75MM, 1KG SPOOL (SHORE 95A)", "TPU", None, "Baby Blue"),
            ("WHITE PA FILAMENT - 1.75MM, 1KG SPOOL", "Nylon", None, "White"),
            ("TRANSPARENT WHITE PC FILAMENT - 1.75MM, 1KG SPOOL", "PC", Some("Transparent"), "White"),
            ("GREEN PERFORMANCE PLA FILAMENT - 1.75MM, 1KG SPOOL", "PLA", Some("Performance"), "Green"),
            ("BLACK PAINT FREE ABS FILAMENT - 1.75MM, 1KG SPOOL", "ABS", Some("Paint Free"), "Black"),
            ("BLACK RAPID PETG FILAMENT \u{2013} 1.75MM, 1KG SPOOL", "PETG", Some("Rapid"), "Black"),
            ("ASH GRAY MATTE PLA FILAMENT - 1.75MM, 1KG SPOOL", "PLA", Some("Matte"), "Ash Gray"),
            ("BLACK PLA MAX V2 FILAMENT - 1.75MM, 1KG SPOOL", "PLA", Some("Max V2"), "Black"),
            ("BLACK PLA PRO+ FILAMENT - 1.75MM, 1KG SPOOL", "PLA", Some("Pro+"), "Black"),
            ("SILK BLACK PLA FILAMENT - 1.75MM, 1KG SPOOL", "PLA", Some("Silk"), "Black"),
            ("METALLIC FINISH GOLD PLA FILAMENT - 1.75MM, 1KG SPOOL", "PLA", Some("Metallic Finish"), "Gold"),
            ("STONE GRANITE ROCK PLA FILAMENT - 1.75MM, 1KG SPOOL", "PLA", Some("Stone"), "Granite Rock"),
            ("STONE GRAY MATTE PLA FILAMENT - 1.75MM, 1KG SPOOL", "PLA", Some("Matte Stone"), "Gray"),
            ("CARBON FIBER PLA FILAMENT - 1.75MM, 1KG SPOOL", "PLA", Some("Carbon Fiber"), "Natural"),
            ("TEMPERATURE COLOR CHANGING PLA FILAMENT - 1.75MM, 1KG SPOOL", "PLA", Some("Temperature Color Changing"), "Natural"),
            ("GLOW IN THE DARK PLA FILAMENT - 1.75MM, 1KG SPOOL", "PLA", Some("Glow In The Dark"), "Natural"),
            ("GLOW IN THE DARK BLUE PLA FILAMENT - 1.75MM, 1KG SPOOL", "PLA", Some("Glow In The Dark"), "Blue"),
            ("UV COLOR CHANGING PURPLE PLA FILAMENT - 1.75MM, 1KG SPOOL", "PLA", Some("UV Color Changing"), "Purple"),
        ];

        for (title, expected_material, expected_variant, expected_color) in cases {
            let (material, variant, color) = parse_product_title(title);
            assert_eq!(material, expected_material, "material for {title:?}");
            assert_eq!(variant, expected_variant.map(str::to_string), "variant for {title:?}");
            assert_eq!(color, expected_color, "color for {title:?}");
        }
    }
}
