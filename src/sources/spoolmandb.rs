use crate::cloak_browser_client::CloakBrowserClient;
use crate::filament_sync_entry::FilamentSyncEntry;
use crate::source::FilamentSource;

const URL: &str = "https://donkie.github.io/SpoolmanDB/filaments.json";

// Every brand our own scrapers don't cover, from SpoolmanDB (specs/003-spoolmandb).
pub struct SpoolmanDbSource;

#[async_trait::async_trait]
impl FilamentSource for SpoolmanDbSource {
    fn name(&self) -> &'static str {
        "spoolmandb"
    }

    async fn fetch(&self, _cloak: Option<&CloakBrowserClient>) -> Result<Vec<FilamentSyncEntry>, String> {
        let json = reqwest::get(URL)
            .await
            .and_then(|r| r.error_for_status())
            .map_err(|e| e.to_string())?
            .text()
            .await
            .map_err(|e| e.to_string())?;
        parse_filaments(&json)
    }
}

// Brands our own scrapers cover — they keep product lines and list more colors (FR-001).
const OWN_BRANDS: &[&str] = &["Bambu Lab", "eSUN", "Hatchbox", "Prusament", "Colorfabb"];
// Brands published before under our own spelling, so spoolbook doesn't see a second brand (FR-002).
const OUR_SPELLINGS: &[&str] = &["Elegoo", "SUNLU", "Polymaker", "Overture", "Creality", "Protopasta", "Fillamentum"];

#[derive(serde::Deserialize)]
struct Filament {
    manufacturer: String,
    material: String,
    name: String,
    color_hex: Option<String>,
    color_hexes: Option<Vec<String>>,
}

fn key(brand: &str) -> String {
    brand.chars().filter(|c| c.is_alphanumeric()).collect::<String>().to_lowercase()
}

fn hex(h: &str) -> String {
    format!("#{}", h.trim_start_matches('#').to_uppercase())
}

pub fn parse_filaments(json: &str) -> Result<Vec<FilamentSyncEntry>, String> {
    let filaments: Vec<Filament> = serde_json::from_str(json).map_err(|e| format!("not a SpoolmanDB filament list: {e}"))?;
    let mut seen = std::collections::HashSet::new();
    Ok(filaments
        .into_iter()
        .filter(|f| !OWN_BRANDS.iter().any(|b| key(b) == key(&f.manufacturer)))
        .filter_map(|f| {
            let brand = OUR_SPELLINGS.iter().find(|b| key(b) == key(&f.manufacturer)).map_or(f.manufacturer.as_str(), |b| b);
            let mut entry = FilamentSyncEntry::new(brand, &f.material, None, &f.name);
            entry.hex = match (&f.color_hexes, &f.color_hex) {
                (Some(list), _) if list.len() >= 2 => Some(list.iter().map(|h| hex(h)).collect::<Vec<_>>().join(", ")),
                (_, Some(h)) => Some(hex(h)),
                (Some(list), None) => list.first().map(|h| hex(h)),
                _ => None,
            };
            // Listed once per spool weight and diameter (FR-004).
            seen.insert((entry.brand.clone(), entry.material.clone(), entry.color.clone())).then_some(entry)
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(brand: &str, material: &str, color: &str, hex: Option<&str>) -> FilamentSyncEntry {
        FilamentSyncEntry { hex: hex.map(str::to_string), ..FilamentSyncEntry::new(brand, material, None, color) }
    }

    // spec: 003/FR-001, 003/FR-003
    #[test]
    fn a_filament_becomes_an_entry_with_its_hex() {
        let json = r#"[
            {"manufacturer": "Polymaker", "material": "PLA", "name": "PolyTerra™ PLA Charcoal Black", "color_hex": "2b2b2b", "color_hexes": null},
            {"manufacturer": "ANYCUBIC", "material": "PLA", "name": "Matte Green-Brown", "color_hex": null, "color_hexes": ["744E10", "009639"]},
            {"manufacturer": "Verbatim", "material": "PETG", "name": "Mystery", "color_hex": null, "color_hexes": null}
        ]"#;
        assert_eq!(
            parse_filaments(json).unwrap(),
            vec![
                entry("Polymaker", "PLA", "PolyTerra™ PLA Charcoal Black", Some("#2B2B2B")),
                entry("ANYCUBIC", "PLA", "Matte Green-Brown", Some("#744E10, #009639")),
                entry("Verbatim", "PETG", "Mystery", None),
            ]
        );
    }

    // spec: 003/FR-001
    #[test]
    fn brands_our_own_scrapers_cover_are_left_out() {
        let json = r#"[
            {"manufacturer": "Bambu Lab", "material": "PLA", "name": "Black", "color_hex": "000000"},
            {"manufacturer": "eSun", "material": "PLA+", "name": "Black", "color_hex": "111214"},
            {"manufacturer": "Hatchbox", "material": "PLA", "name": "Black", "color_hex": "000000"},
            {"manufacturer": "Prusament", "material": "PETG", "name": "Galaxy Black", "color_hex": "222222"},
            {"manufacturer": "colorFabb", "material": "PLA", "name": "Black", "color_hex": "000000"},
            {"manufacturer": "Fiberlogy", "material": "PLA", "name": "Black", "color_hex": "0A0A0A"}
        ]"#;
        let brands: Vec<String> = parse_filaments(json).unwrap().into_iter().map(|e| e.brand).collect();
        assert_eq!(brands, vec!["Fiberlogy"]);
    }

    // spec: 003/FR-002
    #[test]
    fn a_brand_we_published_before_keeps_our_spelling() {
        let json = r#"[
            {"manufacturer": "ELEGOO", "material": "PLA", "name": "Red", "color_hex": "FF0000"},
            {"manufacturer": "Sunlu", "material": "PLA", "name": "Red", "color_hex": "FF0000"},
            {"manufacturer": "add:north", "material": "PLA", "name": "Red", "color_hex": "FF0000"}
        ]"#;
        let brands: Vec<String> = parse_filaments(json).unwrap().into_iter().map(|e| e.brand).collect();
        assert_eq!(brands, vec!["Elegoo", "SUNLU", "add:north"]);
    }

    // spec: 003/FR-004
    #[test]
    fn one_filament_listed_per_weight_and_diameter_becomes_one_entry() {
        let json = r#"[
            {"manufacturer": "Polymaker", "material": "ABS", "name": "PolyLite™ ABS Black", "color_hex": "16161A", "weight": 1000, "diameter": 1.75},
            {"manufacturer": "Polymaker", "material": "ABS", "name": "PolyLite™ ABS Black", "color_hex": "16161A", "weight": 1000, "diameter": 2.85},
            {"manufacturer": "Polymaker", "material": "ABS", "name": "PolyLite™ ABS Black", "color_hex": "16161A", "weight": 3000, "diameter": 1.75}
        ]"#;
        assert_eq!(parse_filaments(json).unwrap().len(), 1);
    }

    #[test]
    fn garbage_is_an_error() {
        assert!(parse_filaments("<html>").is_err());
    }
}
