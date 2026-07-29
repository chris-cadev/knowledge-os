use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::adapter::ImportError;
use super::magic_bytes::{detect_format, DetectedFormat};

pub struct DirectoryImporter {
    pub recursive: bool,
}

impl DirectoryImporter {
    pub fn new(recursive: bool) -> Self {
        Self { recursive }
    }

    pub fn list_files(&self, path: &Path) -> Result<Vec<PathBuf>, ImportError> {
        let mut files = Vec::new();
        if self.recursive {
            for entry in walkdir::WalkDir::new(path) {
                let entry = entry.map_err(|e| ImportError::Io(e.into()))?;
                if entry.file_type().is_file() {
                    files.push(entry.path().to_path_buf());
                }
            }
        } else {
            for entry in std::fs::read_dir(path)? {
                let entry = entry?;
                if entry.file_type()?.is_file() {
                    files.push(entry.path());
                }
            }
        }
        files.sort();
        Ok(files)
    }

    pub fn list_files_with_formats(
        &self,
        path: &Path,
    ) -> Result<Vec<(PathBuf, DetectedFormat)>, ImportError> {
        let files = self.list_files(path)?;
        let mut result = Vec::new();
        for file in files {
            let fmt = detect_format(&file).unwrap_or(DetectedFormat::Unknown);
            result.push((file, fmt));
        }
        Ok(result)
    }
}

/// Compute SHA-256 hash of file contents for conflict detection.
pub fn compute_content_hash(path: &Path) -> Result<String, ImportError> {
    let bytes = std::fs::read(path)?;
    use sha2::Digest;
    let mut hasher = sha2::Sha256::new();
    hasher.update(&bytes);
    let result = hasher.finalize();
    Ok(format!("{:x}", result))
}

/// Check for conflicts: compare hashes against previously imported hashes.
pub fn detect_conflicts(
    files: &[PathBuf],
    previous_hashes: &HashMap<String, String>,
) -> Vec<(PathBuf, bool)> {
    let mut results = Vec::new();
    for file in files {
        if let Ok(hash) = compute_content_hash(file) {
            let is_conflict = previous_hashes.values().any(|h| h == &hash);
            results.push((file.clone(), is_conflict));
        } else {
            results.push((file.clone(), false));
        }
    }
    results
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_directory_lists_files_top_level_only() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("a.txt"), "a").unwrap();
        std::fs::write(dir.path().join("b.txt"), "b").unwrap();
        let sub = dir.path().join("sub");
        std::fs::create_dir(&sub).unwrap();
        std::fs::write(sub.join("c.txt"), "c").unwrap();

        let importer = DirectoryImporter::new(false);
        let files = importer.list_files(dir.path()).unwrap();
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn test_directory_lists_files_recursive() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("a.txt"), "a").unwrap();
        let sub = dir.path().join("sub");
        std::fs::create_dir(&sub).unwrap();
        std::fs::write(sub.join("c.txt"), "c").unwrap();

        let importer = DirectoryImporter::new(true);
        let files = importer.list_files(dir.path()).unwrap();
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn test_conflict_detected_by_content_hash() {
        let dir = TempDir::new().unwrap();
        let f1 = dir.path().join("f1.txt");
        let f2 = dir.path().join("f2.txt");
        std::fs::write(&f1, "same content").unwrap();
        std::fs::write(&f2, "same content").unwrap();

        let mut previous = HashMap::new();
        previous.insert("f1".to_string(), compute_content_hash(&f1).unwrap());

        let results = detect_conflicts(&[f1.clone(), f2.clone()], &previous);
        assert!(results[0].1); // f1 matches itself
        assert!(results[1].1); // f2 has same content
    }
}
