use async_trait::async_trait;
use knowledge_core::features::component::{Component, ComponentType};
use knowledge_core::features::entity::{Entity, EntityType};
use std::path::Path;

use super::adapter::{ImportAdapter, ImportError, ImportResult};

pub struct IcsImporter;

impl Default for IcsImporter {
    fn default() -> Self {
        Self::new()
    }
}

impl IcsImporter {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ImportAdapter for IcsImporter {
    fn can_import(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("ics"))
            .unwrap_or(false)
    }

    async fn import(&self, _path: &Path) -> Result<ImportResult, ImportError> {
        Err(ImportError::Parse(
            "ICS import creates multiple entities. Use import_ical() for multi-entity import."
                .into(),
        ))
    }

    fn supported_extensions(&self) -> &[&str] {
        &["ics"]
    }
}

impl IcsImporter {
    pub async fn import_ical(&self, path: &Path) -> Result<Vec<ImportResult>, ImportError> {
        let content = std::fs::read_to_string(path)?;
        let events: Vec<ImportResult> = content
            .split("BEGIN:VEVENT")
            .filter(|s| s.contains("END:VEVENT"))
            .map(|event_block| {
                let summary = extract_ics_field(event_block, "SUMMARY")
                    .unwrap_or_else(|| "Untitled Event".to_string());
                let description = extract_ics_field(event_block, "DESCRIPTION").unwrap_or_default();
                let dtstart = extract_ics_field(event_block, "DTSTART");
                let dtend = extract_ics_field(event_block, "DTEND");

                let entity = Entity::new(EntityType::new("Event"));
                let mut components = vec![
                    Component::new(entity.id, ComponentType::Title, serde_json::json!(summary)),
                    Component::new(
                        entity.id,
                        ComponentType::Content,
                        serde_json::json!(description),
                    ),
                    Component::new(
                        entity.id,
                        ComponentType::Provenance,
                        serde_json::json!({
                            "source": path.to_string_lossy(),
                            "imported_at": chrono::Utc::now().to_rfc3339(),
                            "format": "ics",
                        }),
                    ),
                ];

                if let Some(start) = dtstart {
                    let mut timeline = serde_json::json!({
                        "imported_at": chrono::Utc::now().to_rfc3339(),
                    });
                    if let Some(obj) = timeline.as_object_mut() {
                        obj.insert("created_at".to_string(), serde_json::json!(start));
                        if let Some(end) = &dtend {
                            obj.insert("ended_at".to_string(), serde_json::json!(end));
                        }
                    }
                    components.push(Component::new(entity.id, ComponentType::Timeline, timeline));
                }

                ImportResult {
                    entity,
                    components,
                    cross_references: vec![],
                }
            })
            .collect();

        Ok(events)
    }
}

fn extract_ics_field(event_block: &str, field: &str) -> Option<String> {
    for line in event_block.lines() {
        let trimmed = line.trim();
        if let Some(after_field) = trimmed.strip_prefix(field) {
            if let Some(val) = after_field.strip_prefix(':') {
                let v = val.trim().to_string();
                if !v.is_empty() {
                    return Some(v);
                }
            } else if after_field.starts_with(';') {
                if let Some(val_start) = after_field.find(':') {
                    let val = after_field[val_start + 1..].trim().to_string();
                    if !val.is_empty() {
                        return Some(val);
                    }
                }
            }
        }
    }
    None
}

pub struct VcfImporter;

impl Default for VcfImporter {
    fn default() -> Self {
        Self::new()
    }
}

impl VcfImporter {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ImportAdapter for VcfImporter {
    fn can_import(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("vcf"))
            .unwrap_or(false)
    }

    async fn import(&self, _path: &Path) -> Result<ImportResult, ImportError> {
        Err(ImportError::Parse(
            "VCF import creates multiple entities. Use import_vcards() for multi-entity import."
                .into(),
        ))
    }

    fn supported_extensions(&self) -> &[&str] {
        &["vcf"]
    }
}

impl VcfImporter {
    pub async fn import_vcards(&self, path: &Path) -> Result<Vec<ImportResult>, ImportError> {
        let content = std::fs::read_to_string(path)?;
        let cards: Vec<ImportResult> = content
            .split("BEGIN:VCARD")
            .filter(|s| s.contains("END:VCARD"))
            .map(|card_block| {
                let name = extract_vcf_field(card_block, "FN")
                    .or_else(|| extract_vcf_field(card_block, "N"))
                    .unwrap_or_else(|| "Untitled Contact".to_string());
                let email = extract_vcf_field(card_block, "EMAIL");
                let tel = extract_vcf_field(card_block, "TEL");
                let org = extract_vcf_field(card_block, "ORG");

                let entity = Entity::new(EntityType::new("Person"));
                let mut components = vec![
                    Component::new(entity.id, ComponentType::Title, serde_json::json!(name)),
                    Component::new(
                        entity.id,
                        ComponentType::Provenance,
                        serde_json::json!({
                            "source": path.to_string_lossy(),
                            "imported_at": chrono::Utc::now().to_rfc3339(),
                            "format": "vcf",
                        }),
                    ),
                ];

                let mut contact_data = serde_json::json!({});
                if let Some(obj) = contact_data.as_object_mut() {
                    if let Some(e) = email {
                        obj.insert("email".to_string(), serde_json::json!(e));
                    }
                    if let Some(t) = tel {
                        obj.insert("phone".to_string(), serde_json::json!(t));
                    }
                    if let Some(o) = org {
                        obj.insert("organization".to_string(), serde_json::json!(o));
                    }
                }
                components.push(Component::new(
                    entity.id,
                    ComponentType::Content,
                    contact_data,
                ));

                ImportResult {
                    entity,
                    components,
                    cross_references: vec![],
                }
            })
            .collect();

        Ok(cards)
    }
}

fn extract_vcf_field(card_block: &str, field: &str) -> Option<String> {
    for line in card_block.lines() {
        let trimmed = line.trim();
        if let Some(after_field) = trimmed.strip_prefix(field) {
            if let Some(val) = after_field.strip_prefix(':') {
                let v = val.trim().to_string();
                if !v.is_empty() {
                    return Some(v);
                }
            } else if after_field.starts_with(';') {
                if let Some(val_start) = after_field.find(':') {
                    let val = after_field[val_start + 1..].trim().to_string();
                    if !val.is_empty() {
                        return Some(val);
                    }
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ics_can_import() {
        let importer = IcsImporter::new();
        assert!(importer.can_import(Path::new("test.ics")));
        assert!(!importer.can_import(Path::new("test.txt")));
    }

    #[test]
    fn test_vcf_can_import() {
        let importer = VcfImporter::new();
        assert!(importer.can_import(Path::new("test.vcf")));
        assert!(!importer.can_import(Path::new("test.txt")));
    }

    #[test]
    fn test_extract_ics_field() {
        let block = "SUMMARY:Test Event\nDTSTART:20240101T120000\n";
        assert_eq!(
            extract_ics_field(block, "SUMMARY"),
            Some("Test Event".to_string())
        );
        assert_eq!(
            extract_ics_field(block, "DTSTART"),
            Some("20240101T120000".to_string())
        );
        assert_eq!(extract_ics_field(block, "LOCATION"), None);
    }

    #[test]
    fn test_extract_vcf_field() {
        let block = "FN:John Doe\nEMAIL:john@example.com\n";
        assert_eq!(extract_vcf_field(block, "FN"), Some("John Doe".to_string()));
        assert_eq!(
            extract_vcf_field(block, "EMAIL"),
            Some("john@example.com".to_string())
        );
    }
}
