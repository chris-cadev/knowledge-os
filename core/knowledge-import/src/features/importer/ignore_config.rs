use ignore::gitignore::{Gitignore, GitignoreBuilder};
use std::path::Path;

pub const DEFAULT_PATTERNS: &[&str] = &[
    ".git/",
    "node_modules/",
    "__pycache__/",
    ".pdm-build/",
    ".venv/",
    "venv/",
    "target/",
    "dist/",
    "build/",
    "*.pyc",
    ".DS_Store",
    "Thumbs.db",
];

pub fn build_gitignore<S: AsRef<str>>(patterns: &[S], root: &Path) -> Gitignore {
    let mut builder = GitignoreBuilder::new(root);
    for p in patterns {
        let _ = builder.add_line(None, p.as_ref());
    }
    builder.build().expect("failed to build gitignore from patterns")
}

pub fn load_ignore_file(path: &Path) -> Option<Gitignore> {
    if !path.is_file() {
        return None;
    }
    let root = path.parent()?;
    let patterns = read_patterns(path)?;
    Some(build_gitignore(&patterns, root))
}

fn read_patterns(path: &Path) -> Option<Vec<String>> {
    let content = std::fs::read_to_string(path).ok()?;
    let patterns: Vec<String> = content
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| l.to_string())
        .collect();
    if patterns.is_empty() {
        return None;
    }
    Some(patterns)
}

pub fn resolve_ignore(import_root: &Path, global_kosignore: Option<&Path>) -> Gitignore {
    if let Some(ref patterns) = read_patterns(&import_root.join(".kosignore")) {
        return build_gitignore(patterns, import_root);
    }
    if let Some(ref patterns) = read_patterns(&import_root.join(".gitignore")) {
        return build_gitignore(patterns, import_root);
    }
    if let Some(path) = global_kosignore {
        if let Some(ref patterns) = read_patterns(path) {
            return build_gitignore(patterns, import_root);
        }
    }
    build_gitignore(DEFAULT_PATTERNS, import_root)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_build_gitignore_excludes_defaults() {
        let dir = TempDir::new().unwrap();
        let gi = build_gitignore(DEFAULT_PATTERNS, dir.path());

        assert!(gi.matched_path_or_any_parents(&dir.path().join(".git"), true).is_ignore());
        assert!(gi.matched_path_or_any_parents(&dir.path().join("node_modules"), true).is_ignore());
        assert!(gi.matched_path_or_any_parents(&dir.path().join("foo.pyc"), false).is_ignore());
        assert!(!gi.matched_path_or_any_parents(&dir.path().join("notes.md"), false).is_ignore());
    }

    #[test]
    fn test_load_ignore_file_returns_none_for_missing() {
        let dir = TempDir::new().unwrap();
        assert!(load_ignore_file(&dir.path().join(".kosignore")).is_none());
    }

    #[test]
    fn test_load_ignore_file_parses_patterns() {
        let dir = TempDir::new().unwrap();
        let kosignore = dir.path().join(".kosignore");
        fs::write(&kosignore, "target/\n*.log\n").unwrap();

        let gi = load_ignore_file(&kosignore).unwrap();
        assert!(gi.matched_path_or_any_parents(&dir.path().join("target"), true).is_ignore());
        assert!(gi.matched_path_or_any_parents(&dir.path().join("debug.log"), false).is_ignore());
        assert!(!gi.matched_path_or_any_parents(&dir.path().join("notes.md"), false).is_ignore());
    }

    #[test]
    fn test_resolve_ignore_prefers_kosignore_over_gitignore() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join(".kosignore"), "*.secret\n").unwrap();
        fs::write(dir.path().join(".gitignore"), "*.log\n").unwrap();

        let gi = resolve_ignore(dir.path(), None);
        assert!(gi.matched_path_or_any_parents(&dir.path().join("data.secret"), false).is_ignore());
        assert!(!gi.matched_path_or_any_parents(&dir.path().join("debug.log"), false).is_ignore());
    }

    #[test]
    fn test_resolve_ignore_falls_back_to_gitignore() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join(".gitignore"), "*.log\n").unwrap();

        let gi = resolve_ignore(dir.path(), None);
        assert!(gi.matched_path_or_any_parents(&dir.path().join("debug.log"), false).is_ignore());
    }

    #[test]
    fn test_resolve_ignore_falls_back_to_global_kosignore() {
        let import_dir = TempDir::new().unwrap();
        let config_dir = TempDir::new().unwrap();
        let global = config_dir.path().join(".kosignore");
        fs::write(&global, "*.tmp\n").unwrap();

        let gi = resolve_ignore(import_dir.path(), Some(&global));
        assert!(gi.matched_path_or_any_parents(&import_dir.path().join("cache.tmp"), false).is_ignore());
    }

    #[test]
    fn test_resolve_ignore_uses_defaults_when_nothing_exists() {
        let dir = TempDir::new().unwrap();
        let gi = resolve_ignore(dir.path(), None);

        assert!(gi.matched_path_or_any_parents(&dir.path().join("node_modules"), true).is_ignore());
        assert!(!gi.matched_path_or_any_parents(&dir.path().join("notes.md"), false).is_ignore());
    }
}
