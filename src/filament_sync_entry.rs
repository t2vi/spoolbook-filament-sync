use serde::Serialize;

// Mirrors the C# FilamentSyncEntry record — field names stay PascalCase because
// spoolbook-rs's filament_catalog_sync.rs parses the published catalog against this exact shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FilamentSyncEntry {
    #[serde(rename = "Brand")]
    pub brand: String,
    #[serde(rename = "Material")]
    pub material: String,
    #[serde(rename = "Variant")]
    pub variant: Option<String>,
    #[serde(rename = "Color")]
    pub color: String,
    #[serde(rename = "Hex")]
    pub hex: Option<String>,
}

impl FilamentSyncEntry {
    pub fn new(brand: &str, material: &str, variant: Option<String>, color: &str) -> Self {
        Self {
            brand: brand.to_string(),
            material: material.to_string(),
            variant,
            color: color.to_string(),
            hex: None,
        }
    }
}
