use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::adapter::ImportError;
use super::magic_bytes::{detect_format, DetectedFormat};

pub struct DirectoryImporter {
    pub recursive: bool,
    pub ignore: Option<ignore::gitignore::Gitignore>,
}

impl DirectoryImporter {
    pub fn new(recursive: bool) -> Self {
        Self {
            recursive,
            ignore: None,
        }
    }

    pub fn with_ignore(mut self, gi: ignore::gitignore::Gitignore) -> Self {
        self.ignore = Some(gi);
        self
    }

    pub fn list_files(&self, path: &Path) -> Result<Vec<PathBuf>, ImportError> {
        let mut files = Vec::new();
        if self.recursive {
            if let Some(ref gi) = self.ignore {
                for entry in walkdir::WalkDir::new(path).into_iter().filter_entry(|e| {
                    let is_dir = e.file_type().is_dir();
                    let matched = gi.matched_path_or_any_parents(e.path(), is_dir);
                    !matched.is_ignore()
                }) {
                    let entry = match entry {
                        Ok(e) => e,
                        Err(e) => {
                            eprintln!("Warning: skipping unreadable entry: {e}");
                            continue;
                        }
                    };
                    if entry.file_type().is_file() {
                        files.push(entry.path().to_path_buf());
                    }
                }
            } else {
                for entry in walkdir::WalkDir::new(path) {
                    let entry = match entry {
                        Ok(e) => e,
                        Err(e) => {
                            eprintln!("Warning: skipping unreadable entry: {e}");
                            continue;
                        }
                    };
                    if entry.file_type().is_file() {
                        files.push(entry.path().to_path_buf());
                    }
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

    #[test]
    fn test_ignore_excludes_matching_directories() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("a.md"), "a").unwrap();
        let node_modules = dir.path().join("node_modules");
        std::fs::create_dir(&node_modules).unwrap();
        std::fs::write(node_modules.join("pkg.js"), "js").unwrap();

        let gi = super::super::ignore_config::build_gitignore(&["node_modules/"], dir.path());
        let importer = DirectoryImporter::new(true).with_ignore(gi);
        let files = importer.list_files(dir.path()).unwrap();
        assert_eq!(files.len(), 1);
        assert!(files[0].ends_with("a.md"));
    }

    #[test]
    fn test_ignore_excludes_matching_files() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("a.md"), "a").unwrap();
        std::fs::write(dir.path().join("cache.pyc"), "pyc").unwrap();

        let gi = super::super::ignore_config::build_gitignore(&["*.pyc"], dir.path());
        let importer = DirectoryImporter::new(true).with_ignore(gi);
        let files = importer.list_files(dir.path()).unwrap();
        assert_eq!(files.len(), 1);
        assert!(files[0].ends_with("a.md"));
    }

    #[test]
    fn test_skip_on_permission_denied() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("a.txt"), "a").unwrap();
        let locked = dir.path().join("locked");
        std::fs::create_dir(&locked).unwrap();
        std::fs::write(locked.join("secret.txt"), "secret").unwrap();

        let mut perms = std::fs::metadata(&locked).unwrap().permissions();
        perms.set_readonly(true);
        std::fs::set_permissions(&locked, perms.clone()).unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            perms.set_mode(0o000);
            std::fs::set_permissions(&locked, perms).unwrap();
        }

        let importer = DirectoryImporter::new(true);
        let files = importer.list_files(dir.path()).unwrap();
        assert!(files.iter().any(|f| f.ends_with("a.txt")));

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&locked).unwrap().permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&locked, perms).unwrap();
        }
    }
}
