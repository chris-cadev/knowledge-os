use async_trait::async_trait;
use std::path::Path;

use super::adapter::{ImportAdapter, ImportError, ImportResult};
use super::markdown::MarkdownImporter;

pub struct ObsidianVaultImporter;

impl Default for ObsidianVaultImporter {
    fn default() -> Self {
        Self::new()
    }
}

impl ObsidianVaultImporter {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ImportAdapter for ObsidianVaultImporter {
    fn can_import(&self, path: &Path) -> bool {
        path.is_dir()
    }

    async fn import(&self, _path: &Path) -> Result<ImportResult, ImportError> {
        Err(ImportError::Parse(
            "Obsidian vault import creates multiple entities. Use import_vault() for multi-entity import."
                .into(),
        ))
    }

    fn supported_extensions(&self) -> &[&str] {
        &[]
    }
}

impl ObsidianVaultImporter {
    pub async fn import_vault(&self, path: &Path) -> Result<Vec<ImportResult>, ImportError> {
        if !path.is_dir() {
            return Err(ImportError::Parse(format!(
                "Not a directory: {}",
                path.display()
            )));
        }

        let md_importer = MarkdownImporter::new();
        let mut results = Vec::new();

        let entries = std::fs::read_dir(path)?;
        for entry in entries {
            let entry = entry?;
            let entry_path = entry.path();
            if entry_path.is_file()
                && entry_path
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|e| e.eq_ignore_ascii_case("md"))
                    .unwrap_or(false)
            {
                match md_importer.import(&entry_path).await {
                    Ok(result) => results.push(result),
                    Err(e) => {
                        eprintln!("Warning: Failed to import {}: {}", entry_path.display(), e);
                    }
                }
            }
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_can_import_directory() {
        let importer = ObsidianVaultImporter::new();
        assert!(importer.can_import(Path::new("/tmp")));
        assert!(!importer.can_import(Path::new("nonexistent.md")));
    }

    #[tokio::test]
    async fn test_obsidian_imports_directory() {
        let dir = TempDir::new().unwrap();
        let file1_path = dir.path().join("note1.md");
        let file2_path = dir.path().join("note2.md");
        let mut f1 = std::fs::File::create(&file1_path).unwrap();
        f1.write_all(b"# Note 1\nContent 1").unwrap();
        let mut f2 = std::fs::File::create(&file2_path).unwrap();
        f2.write_all(b"# Note 2\nContent 2").unwrap();

        let importer = ObsidianVaultImporter::new();
        let results = importer.import_vault(dir.path()).await.unwrap();
        assert_eq!(results.len(), 2);
        let titles: Vec<String> = results
            .iter()
            .filter_map(|r| {
                r.components
                    .iter()
                    .find(|c| {
                        c.component_type
                            == knowledge_core::features::component::ComponentType::Title
                    })
                    .map(|c| c.data.as_str().unwrap_or("").to_string())
            })
            .collect();
        assert!(titles.contains(&"Note 1".to_string()));
        assert!(titles.contains(&"Note 2".to_string()));
    }
}
