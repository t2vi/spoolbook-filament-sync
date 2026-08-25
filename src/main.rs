use spoolbook_filament_sync::cloak_browser_client::CloakBrowserClient;
use spoolbook_filament_sync::color_hex_resolver;
use spoolbook_filament_sync::filament_sync_entry::FilamentSyncEntry;
use spoolbook_filament_sync::source::FilamentSource;
use spoolbook_filament_sync::sources::{
    BambuSource, ColorfabbSource, CrealitySource, ElegooSource, EsunSource, FillamentumSource, HatchboxSource,
    OvertureSource, PolymakerSource, ProtopastaSource, PrusamentSource, Slic3DSource, SunluSource,
};
use std::collections::HashMap;

// Registry of ported brands — all 13 done (spoolbook-filament-sync#1).
fn all_sources() -> Vec<Box<dyn FilamentSource>> {
    vec![
        Box::new(ElegooSource),
        Box::new(SunluSource),
        Box::new(PolymakerSource),
        Box::new(OvertureSource),
        Box::new(CrealitySource),
        Box::new(PrusamentSource),
        Box::new(ProtopastaSource),
        Box::new(ColorfabbSource),
        Box::new(FillamentumSource),
        Box::new(EsunSource),
        Box::new(BambuSource),
        Box::new(HatchboxSource),
        Box::new(Slic3DSource),
    ]
}

// Only Hatchbox/Slic3D need it — shared across both rather than one browser instance each.
const CLOAK_BRANDS: &[&str] = &["hatchbox", "slic3d"];

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
    for source in selected {
        println!("Fetching {}...", source.name());
        match source.fetch(cloak.as_ref()).await {
            Ok(mut e) => entries.append(&mut e),
            Err(err) => {
                eprintln!("Skipping {}: {err}", source.name());
                continue;
            }
        }
    }

    let deduped = dedupe_and_resolve_hex(entries);

    let json = serde_json::to_string_pretty(&deduped).expect("serialize entries");
    if let Err(e) = std::fs::write(output_path, &json) {
        eprintln!("Failed to write {output_path}: {e}");
        return std::process::ExitCode::FAILURE;
    }

    println!("Wrote {} entries to {output_path}", deduped.len());
    std::process::ExitCode::SUCCESS
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

    let mut hex_by_color: HashMap<String, Option<String>> = HashMap::new();
    for entry in &deduped {
        hex_by_color
            .entry(entry.color.clone())
            .or_insert_with(|| color_hex_resolver::resolve(&entry.color));
    }

    deduped
        .into_iter()
        .map(|mut e| {
            e.hex = hex_by_color.get(&e.color).cloned().flatten();
            e
        })
        .collect()
}
