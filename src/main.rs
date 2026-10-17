use spoolbook_filament_sync::cloak_browser_client::CloakBrowserClient;
use spoolbook_filament_sync::color_hex_resolver;
use spoolbook_filament_sync::filament_sync_entry::FilamentSyncEntry;
use spoolbook_filament_sync::source::FilamentSource;
use spoolbook_filament_sync::sources::{
    BambuSource, ColorfabbSource, EsunSource, HatchboxSource, PrusamentSource, SpoolmanDbSource,
};
use std::collections::HashMap;

// SpoolmanDB for every brand but the five our own scrapers reach and describe better
// (specs/002-sources FR-004, 003-spoolmandb). The Shopify stores and Jaycar block GitHub's runners.
fn all_sources() -> Vec<Box<dyn FilamentSource>> {
    vec![
        Box::new(SpoolmanDbSource),
        Box::new(PrusamentSource),
        Box::new(ColorfabbSource),
        Box::new(EsunSource),
        Box::new(BambuSource),
        Box::new(HatchboxSource),
    ]
}

// Only Hatchbox needs it.
const CLOAK_BRANDS: &[&str] = &["hatchbox"];

#[tokio::main]
async fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let known: Vec<&str> = all_sources().iter().map(|s| s.name()).collect();

    if args.len() < 3 {
        eprintln!(
            "Usage: spoolbook-filament-sync <output-json-path> <all|{}>",
            known.join("|")
        );
        return std::process::ExitCode::FAILURE;
    }

    let output_path = &args[1];
    let requested = args[2].as_str();

    let sources = all_sources();
    let selected: Vec<&Box<dyn FilamentSource>> = if requested == "all" {
        sources.iter().collect()
    } else {
        match sources.iter().find(|s| s.name() == requested) {
            Some(s) => vec![s],
            None => {
                eprintln!("Unknown source '{requested}'. Expected 'all' or one of: {}", known.join(", "));
                return std::process::ExitCode::FAILURE;
            }
        }
    };

    // CloakBrowser only spins up when a selected source actually needs it.
    let needs_cloak = selected.iter().any(|s| CLOAK_BRANDS.contains(&s.name()));
    let cloak: Option<CloakBrowserClient> = if needs_cloak {
        match std::env::var("CLOAKBROWSER_WS_URL") {
            Ok(url) => Some(CloakBrowserClient::new(url)),
            Err(_) => {
                eprintln!("CLOAKBROWSER_WS_URL is not set — CF-dependent scrapers will not work");
                return std::process::ExitCode::FAILURE;
            }
        }
    } else {
        None
    };

    let mut entries: Vec<FilamentSyncEntry> = Vec::new();
    let mut scraped = 0;
    for source in &selected {
        println!("Fetching {}...", source.name());
        match source_outcome(source.fetch(cloak.as_ref()).await) {
            Ok(mut e) => {
                scraped += 1;
                entries.append(&mut e);
            }
            Err(err) => {
                eprintln!("Skipping {}: {err}", source.name());
                println!("{}", skip_annotation(source.name(), &err));
            }
        }
    }
    println!("{}", scrape_summary(scraped, selected.len()));

    let deduped = dedupe_and_resolve_hex(entries);

    let json = serde_json::to_string_pretty(&deduped).expect("serialize entries");
    if let Err(e) = std::fs::write(output_path, &json) {
        eprintln!("Failed to write {output_path}: {e}");
        return std::process::ExitCode::FAILURE;
    }

    println!("Wrote {} entries to {output_path}", deduped.len());
    std::process::ExitCode::SUCCESS
}

// A source that finds nothing failed as surely as one that errored — a blocked or changed page
// parses to zero products (specs/002-sources FR-002).
fn source_outcome(result: Result<Vec<FilamentSyncEntry>, String>) -> Result<Vec<FilamentSyncEntry>, String> {
    match result {
        Ok(entries) if entries.is_empty() => Err("no entries found".to_string()),
        other => other,
    }
}

// GitHub Actions turns this stdout line into a warning on the run's summary page.
fn skip_annotation(brand: &str, reason: &str) -> String {
    format!("::warning title=Brand skipped::{brand}: {reason}")
}

fn scrape_summary(scraped: usize, total: usize) -> String {
    format!("Scraped {scraped} of {total} sources")
}

// Multi-pack/bundle SKUs (e.g. "PLA-Basic 4 Rolls") re-list the same product's colors under
// the same Material/Variant as the single-roll listing — dedupe rather than maintain a
// growing per-site blocklist of bundle slugs. Resolve hex once per unique color name rather
// than per entry — the same name repeats across many brands/materials/variants.
fn dedupe_and_resolve_hex(entries: Vec<FilamentSyncEntry>) -> Vec<FilamentSyncEntry> {
    let mut seen = std::collections::HashSet::new();
    let deduped: Vec<FilamentSyncEntry> = entries
        .into_iter()
        .filter(|e| seen.insert((e.brand.clone(), e.material.clone(), e.variant.clone(), e.color.clone())))
        .collect();

    // A hex the source gave (SpoolmanDB) is kept; the rest resolve from the name (001/FR-013).
    let mut hex_by_color: HashMap<String, Option<String>> = HashMap::new();
    for entry in deduped.iter().filter(|e| e.hex.is_none()) {
        hex_by_color
            .entry(entry.color.clone())
            .or_insert_with(|| color_hex_resolver::resolve_hex(&entry.color));
    }

    deduped
        .into_iter()
        .map(|mut e| {
            if e.hex.is_none() {
                e.hex = hex_by_color.get(&e.color).cloned().flatten();
            }
            e
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // spec: 001/FR-013, 001/FR-010
    // spec: 002/FR-004, 003/FR-005
    #[test]
    fn the_scrape_runs_spoolmandb_and_the_five_store_scrapers() {
        let mut names: Vec<&str> = all_sources().iter().map(|s| s.name()).collect();
        names.sort();
        assert_eq!(names, vec!["bambu", "colorfabb", "esun", "hatchbox", "prusament", "spoolmandb"]);
    }

    // spec: 001/FR-013
    #[test]
    fn an_entry_that_brought_its_own_hex_keeps_it() {
        let mut real = FilamentSyncEntry::new("Fiberlogy", "PLA", None, "Black");
        real.hex = Some("#0A0A0A".to_string());
        let guessed = FilamentSyncEntry::new("Hatchbox", "PLA", None, "Black");
        let resolved = dedupe_and_resolve_hex(vec![real, guessed]);
        assert_eq!(resolved[0].hex.as_deref(), Some("#0A0A0A"));
        assert_eq!(resolved[1].hex.as_deref(), Some("#000000"));
    }

    // spec: 002/FR-001, 002/FR-002
    #[test]
    fn a_source_that_errors_or_finds_nothing_is_skipped_with_its_reason() {
        let entry = FilamentSyncEntry::new("A", "PLA", None, "Black");
        assert_eq!(source_outcome(Ok(vec![entry.clone()])), Ok(vec![entry]));
        assert_eq!(source_outcome(Ok(vec![])), Err("no entries found".to_string()));
        assert_eq!(source_outcome(Err("HTTP 429".to_string())), Err("HTTP 429".to_string()));
    }

    // spec: 002/FR-002
    #[test]
    fn a_skipped_source_is_reported_as_a_warning_annotation() {
        assert_eq!(skip_annotation("elegoo", "HTTP 429"), "::warning title=Brand skipped::elegoo: HTTP 429");
        assert_eq!(scrape_summary(5, 14), "Scraped 5 of 14 sources");
    }

    #[test]
    fn every_entry_with_the_same_color_name_gets_the_same_hex() {
        let entries = vec![
            FilamentSyncEntry::new("A", "PLA", None, "Black+Gold"),
            FilamentSyncEntry::new("B", "PETG", Some("Silk".to_string()), "Black+Gold"),
            FilamentSyncEntry::new("B", "PETG", Some("Silk".to_string()), "Black+Gold"),
        ];
        let resolved = dedupe_and_resolve_hex(entries);
        assert_eq!(resolved.len(), 2);
        assert!(resolved.iter().all(|e| e.hex.as_deref() == Some("#000000, #FFD700")));
    }
}
