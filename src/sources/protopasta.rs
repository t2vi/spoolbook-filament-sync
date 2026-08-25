use crate::cloak_browser_client::CloakBrowserClient;
use crate::filament_sync_entry::FilamentSyncEntry;
use crate::source::FilamentSource;
use regex::Regex;
use serde::Deserialize;
use std::sync::LazyLock;

// Single request only (Shopify's products.json returns the whole collection at once) — the bare
// domain proto-pasta.com is a plain Shopify theme with no anti-bot wall.
const BASE_URL: &str = "https://proto-pasta.com";

pub struct ProtopastaSource;

#[async_trait::async_trait]
impl FilamentSource for ProtopastaSource {
    fn name(&self) -> &'static str {
        "protopasta"
    }

    async fn fetch(&self, _cloak: Option<&CloakBrowserClient>) -> Result<Vec<FilamentSyncEntry>, String> {
        let json = fetch_collection().await?;
        let titles = filter_real_products(parse_collection(&json));

        Ok(titles
            .into_iter()
            .map(|title| {
                let (material, variant, color) = parse_product_title(&title);
                FilamentSyncEntry::new("Protopasta", &material, variant, &color)
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

#[derive(Deserialize)]
struct ShopifyCollection {
    products: Vec<ShopifyProduct>,
}
#[derive(Deserialize)]
struct ShopifyProduct {
    title: String,
    product_type: String,
}

// Every color is its own standalone product (no per-product color options), so the listing
// title alone is enough — no per-product page fetch needed.
fn parse_collection(json: &str) -> Vec<String> {
    let data: ShopifyCollection = match serde_json::from_str(json) {
        Ok(d) => d,
        Err(_) => return Vec::new(),
    };
    data.products
        .into_iter()
        .filter(|p| p.product_type == "3D Printer Filament")
        .map(|p| p.title)
        .collect()
}

// Subscription boxes aren't single-color SKUs; the charity/collab one-off and the two
// "Glow-in-the-Dark" exclusives state no material anywhere (title or description) so there's
// nothing honest to classify them as.
const EXCLUDE_PATTERNS: &[&str] = &["Subscription", "Glow-in-the-Dark"];
const EXCLUDE_EXACT: &str = "\"We Keep Us Safe\" by The Whistle Crew";

fn filter_real_products(titles: Vec<String>) -> Vec<String> {
    titles
        .into_iter()
        .filter(|t| t != EXCLUDE_EXACT && !EXCLUDE_PATTERNS.iter().any(|p| t.contains(p)))
        .collect()
}

// Longest/most-specific first so "HTPLA"/"HFPLA" match before the bare "PLA" they contain.
const KNOWN_MATERIALS: &[&str] = &["HTPLA", "HFPLA", "PCTG", "PETG", "TPU", "TPE", "Polyketone", "PLA"];

// Product-line descriptors that sit next to the material/color rather than being a color
// themselves — stripped from both ends of the remaining text (iteratively, since some products
// stack more than one, e.g. "Static Dissipative Carbon Fiber"), longest first.
const VARIANT_QUALIFIERS: &[&str] = &[
    "High Strength Carbon Fiber", "High Impact Carbon Fiber", "Carbon Fiber Composite",
    "Recycled Carbon Fiber", "Metal Composite", "Glass Fiber", "Carbon Fiber",
    "Static Dissipative", "Quantum Dot", "Ice Translucent", "Electrically Conductive",
    "High Density", "High Flow", "Matte Fiber", "Glitter Flake",
    "Multicolor", "Translucent", "Metallic", "Glitter", "Opaque", "Reflective",
    "Thermochromic", "c-Matte", "Marble", "Rigid", "Flexible", "Premium Basic",
    "Recycled", "Composite", "Smooth",
];

static TRAILING_PROMO_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s*\*[^*]+\*\s*$").unwrap());
static HFPLA_PAREN_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s*\(HFPLA\)").unwrap());
static PREMIUM_BASIC_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(.+?)\s+PLA\s*-\s*Premium Basic PLA$").unwrap());
static MATTE_FIBER_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^Matte Fiber (HTPLA|PLA)\s*-\s*(.+)$").unwrap());
static STAINLESS_STEEL_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^Stainless Steel Metal Composite PLA\s*-\s*(.+?)\s*color$").unwrap());
// "{Color} [Material] with {Accent} Glitter" — the accent is part of the color's own name, not
// a separate product line, and these specialty one-offs are always Protopasta's HTPLA
// (confirmed via product descriptions) even on the few titles that omit the material word.
static WITH_ACCENT_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(.+?)(?:\s+(HTPLA|PLA))?\s+with\s+(.+)$").unwrap());
static FILLED_SUFFIX_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(.*?)-filled$").unwrap());

fn strip_qualifiers(mut text: String) -> (String, Vec<&'static str>) {
    let mut matched: Vec<&'static str> = Vec::new();
    loop {
        let mut changed = false;
        for q in VARIANT_QUALIFIERS {
            if text == *q {
                text = String::new();
                matched.push(q);
                changed = true;
                break;
            }
            if let Some(stripped) = text.strip_suffix(&format!(" {q}")) {
                text = stripped.to_string();
                matched.push(q);
                changed = true;
                break;
            }
            if let Some(stripped) = text.strip_prefix(&format!("{q} ")) {
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

fn parse_product_title(raw_title: &str) -> (String, Option<String>, String) {
    let t = TRAILING_PROMO_RE.replace(raw_title.trim(), "");
    let t = HFPLA_PAREN_RE.replace(&t, "").trim().to_string();

    if let Some(caps) = PREMIUM_BASIC_RE.captures(&t) {
        return ("PLA".to_string(), Some("Premium Basic".to_string()), caps[1].trim().to_string());
    }

    if let Some(caps) = MATTE_FIBER_RE.captures(&t) {
        return (caps[1].to_string(), Some("Matte Fiber".to_string()), caps[2].trim().to_string());
    }

    if let Some(caps) = STAINLESS_STEEL_RE.captures(&t) {
        return ("PLA".to_string(), Some("Stainless Steel Metal Composite".to_string()), caps[1].trim().to_string());
    }

    if let Some(caps) = WITH_ACCENT_RE.captures(&t) {
        let material = caps.get(2).map(|m| m.as_str()).unwrap_or("HTPLA").to_string();
        return (material, None, format!("{} with {}", caps[1].trim(), caps[3].trim()));
    }

    let mut matched_material: Option<&str> = None;
    let mut remainder = t.clone();
    for material in KNOWN_MATERIALS {
        let re = Regex::new(&format!(r"\b{}\b", regex::escape(material))).unwrap();
        if let Some(m) = re.find(&t) {
            matched_material = Some(material);
            let joined = format!("{} {}", &t[..m.start()], &t[m.end()..]).trim().to_string();
            static WHITESPACE_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s+").unwrap());
            remainder = WHITESPACE_RE.replace_all(&joined, " ").to_string();
            break;
        }
    }

    let Some(matched_material) = matched_material else {
        return (t.clone(), None, t);
    };

    let (mut color_text, matched_qualifiers) = strip_qualifiers(remainder);
    if let Some(caps) = FILLED_SUFFIX_RE.captures(&color_text.clone()) {
        color_text = caps[1].trim().to_string();
    }

    let variant = if matched_qualifiers.is_empty() {
        None
    } else {
        Some(matched_qualifiers.into_iter().rev().collect::<Vec<_>>().join(" "))
    };

    (
        matched_material.to_string(),
        variant,
        if color_text.is_empty() { "Natural".to_string() } else { color_text },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_real_products_excludes_non_filament_and_ambiguous_products() {
        let titles = vec![
            "Black c-Matte PLA".to_string(),
            "Endless PLA Filament Color Subscription *2026 update*".to_string(),
            "\"We Keep Us Safe\" by The Whistle Crew".to_string(),
            "Green Glow-in-the-Dark, Natural".to_string(),
        ];
        assert_eq!(filter_real_products(titles), vec!["Black c-Matte PLA".to_string()]);
    }

    #[test]
    fn parse_product_title_splits_material_variant_color() {
        let cases: &[(&str, &str, Option<&str>, &str)] = &[
            ("Green Quantum Dot HTPLA", "HTPLA", Some("Quantum Dot"), "Green"),
            ("Yellow Reflective HTPLA", "HTPLA", Some("Reflective"), "Yellow"),
            ("Fluorescent Yellow Reflective HTPLA", "HTPLA", Some("Reflective"), "Fluorescent Yellow"),
            ("Gradient Gray Multicolor HTPLA", "HTPLA", Some("Multicolor"), "Gradient Gray"),
            ("Jurassic Jungle Green c-Matte PLA", "PLA", Some("c-Matte"), "Jurassic Jungle Green"),
            ("Clear PCTG", "PCTG", None, "Clear"),
            ("Obsidian HTPLA", "HTPLA", None, "Obsidian"),
            ("Stef's Rose Gold HTPLA", "HTPLA", None, "Stef's Rose Gold"),
            ("Black High Strength Carbon Fiber PCTG", "PCTG", Some("High Strength Carbon Fiber"), "Black"),
            ("Light Gray Carbon Fiber Composite HTPLA", "HTPLA", Some("Carbon Fiber Composite"), "Light Gray"),
            ("Static Dissipative Carbon Fiber PLA", "PLA", Some("Static Dissipative Carbon Fiber"), "Natural"),
            ("Copper-filled Metal Composite HTPLA", "HTPLA", Some("Metal Composite"), "Copper"),
            ("High Density Iron-filled HTPLA", "HTPLA", Some("High Density"), "Iron"),
            ("Natural High Flow PLA (HFPLA)", "PLA", Some("High Flow"), "Natural"),
            ("Clear PETG *new low price*", "PETG", None, "Clear"),
            ("Black PLA - Premium Basic PLA", "PLA", Some("Premium Basic"), "Black"),
            ("Matte Fiber HTPLA - Daffodil Wood", "HTPLA", Some("Matte Fiber"), "Daffodil Wood"),
            ("Stainless Steel Metal Composite PLA - Blue color", "PLA", Some("Stainless Steel Metal Composite"), "Blue"),
            ("Night Before Blue HTPLA with Silver Glitter", "HTPLA", None, "Night Before Blue with Silver Glitter"),
            ("Texas Tea Black with Gold Glitter", "HTPLA", None, "Texas Tea Black with Gold Glitter"),
        ];
        for (title, mat, var, color) in cases {
            let (m, v, c) = parse_product_title(title);
            assert_eq!(&m, mat, "material for {title:?}");
            assert_eq!(v.as_deref(), *var, "variant for {title:?}");
            assert_eq!(&c, color, "color for {title:?}");
        }
    }
}
