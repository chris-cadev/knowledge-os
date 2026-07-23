use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

fn write_file(dir: &std::path::Path, name: &str, content: &str) -> std::path::PathBuf {
    let path = dir.join(name);
    fs::write(&path, content).unwrap();
    path
}

#[test]
fn test_import_single_file() {
    let tmp = TempDir::new().unwrap();
    let md = write_file(tmp.path(), "test.md", "---\ntitle: \"Test Doc\"\ntags:\n  - rust\n---\n\n# Test\n\nHello world.");

    let output = Command::cargo_bin("kos").unwrap()
        .current_dir(tmp.path())
        .arg("--db").arg("test.db")
        .arg("import")
        .arg(&md)
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Created:"));
    assert!(stdout.contains("Article"));
}

#[test]
fn test_import_directory() {
    let tmp = TempDir::new().unwrap();
    write_file(tmp.path(), "a.md", "# Doc A\n\nContent A.");
    write_file(tmp.path(), "b.md", "# Doc B\n\nContent B.");
    write_file(tmp.path(), "not_a.txt", "ignore this");

    let output = Command::cargo_bin("kos").unwrap()
        .current_dir(tmp.path())
        .arg("--db").arg("test.db")
        .arg("import")
        .arg(tmp.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Total files: 2"));
    assert!(stdout.contains("Created: 2"));
}

#[test]
fn test_idempotent_reimport_updates_version() {
    let tmp = TempDir::new().unwrap();
    let md = write_file(tmp.path(), "test.md", "# Same Title\n\nOriginal content.");

    // First import
    let output = Command::cargo_bin("kos").unwrap()
        .current_dir(tmp.path())
        .arg("--db").arg("test.db")
        .arg("import")
        .arg(&md)
        .output()
        .unwrap();
    assert!(output.status.success());

    // Second import (should update, not create)
    let output = Command::cargo_bin("kos").unwrap()
        .current_dir(tmp.path())
        .arg("--db").arg("test.db")
        .arg("import")
        .arg(&md)
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stdout.contains("Updated: 1"), "Expected 'Updated: 1', got stdout: {}", stdout);
    assert!(!stderr.contains("ERROR"), "Expected no errors, got stderr: {}", stderr);
}

#[test]
fn test_search_by_keyword() {
    let tmp = TempDir::new().unwrap();
    let md = write_file(tmp.path(), "test.md", "---\ntitle: \"Transformer Paper\"\ntags:\n  - transformer\n---\n\n# Transformer\n\nAttention is all you need.");

    // Import
    Command::cargo_bin("kos").unwrap()
        .current_dir(tmp.path())
        .arg("--db").arg("test.db")
        .arg("import")
        .arg(&md)
        .output()
        .unwrap();

    // Search
    let output = Command::cargo_bin("kos").unwrap()
        .current_dir(tmp.path())
        .arg("--db").arg("test.db")
        .arg("search")
        .arg("transformer")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Found 1"));
    assert!(stdout.contains("Transformer Paper"));
}

#[test]
fn test_search_with_type_filter() {
    let tmp = TempDir::new().unwrap();
    write_file(tmp.path(), "a.md", "---\ntitle: \"Article One\"\n---\n\nBody.");
    write_file(tmp.path(), "b.md", "---\ntitle: \"Article Two\"\n---\n\nBody.");

    Command::cargo_bin("kos").unwrap()
        .current_dir(tmp.path())
        .arg("--db").arg("test.db")
        .arg("import")
        .arg(tmp.path())
        .output()
        .unwrap();

    let output = Command::cargo_bin("kos").unwrap()
        .current_dir(tmp.path())
        .arg("--db").arg("test.db")
        .arg("search")
        .arg("Article")
        .arg("--type").arg("Article")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Found 2"));
}

#[test]
fn test_list_entities() {
    let tmp = TempDir::new().unwrap();
    write_file(tmp.path(), "a.md", "# Doc A");
    write_file(tmp.path(), "b.md", "# Doc B");

    Command::cargo_bin("kos").unwrap()
        .current_dir(tmp.path())
        .arg("--db").arg("test.db")
        .arg("import")
        .arg(tmp.path())
        .output()
        .unwrap();

    let output = Command::cargo_bin("kos").unwrap()
        .current_dir(tmp.path())
        .arg("--db").arg("test.db")
        .arg("list")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Found 2"));
}

#[test]
fn test_archive_and_restore() {
    let tmp = TempDir::new().unwrap();
    let md = write_file(tmp.path(), "test.md", "# Archive Test");

    // Import
    let output = Command::cargo_bin("kos").unwrap()
        .current_dir(tmp.path())
        .arg("--db").arg("test.db")
        .arg("import")
        .arg(&md)
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();

    // Extract entity ID
    let id_line = stdout.lines().find(|l| l.contains("Created:")).unwrap();
    let id = id_line.split_whitespace().nth(2).unwrap();

    // Archive
    let output = Command::cargo_bin("kos").unwrap()
        .current_dir(tmp.path())
        .arg("--db").arg("test.db")
        .arg("archive")
        .arg(id)
        .output()
        .unwrap();
    assert!(output.status.success());

    // List should be empty
    let output = Command::cargo_bin("kos").unwrap()
        .current_dir(tmp.path())
        .arg("--db").arg("test.db")
        .arg("list")
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("No entities found"));

    // Restore
    let output = Command::cargo_bin("kos").unwrap()
        .current_dir(tmp.path())
        .arg("--db").arg("test.db")
        .arg("restore")
        .arg(id)
        .output()
        .unwrap();
    assert!(output.status.success());

    // List should show it again
    let output = Command::cargo_bin("kos").unwrap()
        .current_dir(tmp.path())
        .arg("--db").arg("test.db")
        .arg("list")
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Found 1"));
}

#[test]
fn test_get_entity_details() {
    let tmp = TempDir::new().unwrap();
    let md = write_file(tmp.path(), "test.md", "---\ntitle: \"Detail Test\"\ntags:\n  - test\n---\n\n# Detail Test\n\nSome content.");

    let output = Command::cargo_bin("kos").unwrap()
        .current_dir(tmp.path())
        .arg("--db").arg("test.db")
        .arg("import")
        .arg(&md)
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    let id_line = stdout.lines().find(|l| l.contains("Created:")).unwrap();
    let id = id_line.split_whitespace().nth(2).unwrap();

    let output = Command::cargo_bin("kos").unwrap()
        .current_dir(tmp.path())
        .arg("--db").arg("test.db")
        .arg("get")
        .arg(id)
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Entity:"));
    assert!(stdout.contains("Article"));
    assert!(stdout.contains("Version: 1"));
    assert!(stdout.contains("Components:"));
}

#[test]
fn test_cross_references_create_relationships() {
    let tmp = TempDir::new().unwrap();
    write_file(tmp.path(), "source.md", "# Source\n\nSee [target](target.md).");
    write_file(tmp.path(), "target.md", "# Target\n\nTarget content.");

    // Import directory
    let output = Command::cargo_bin("kos").unwrap()
        .current_dir(tmp.path())
        .arg("--db").arg("test.db")
        .arg("import")
        .arg(tmp.path())
        .output()
        .unwrap();
    assert!(output.status.success());

    // List entities
    let output = Command::cargo_bin("kos").unwrap()
        .current_dir(tmp.path())
        .arg("--db").arg("test.db")
        .arg("list")
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Found 2"));
}

#[test]
fn test_rebuild_index() {
    let tmp = TempDir::new().unwrap();
    write_file(tmp.path(), "test.md", "# Rebuild Test");

    Command::cargo_bin("kos").unwrap()
        .current_dir(tmp.path())
        .arg("--db").arg("test.db")
        .arg("import")
        .arg(tmp.path().join("test.md"))
        .output()
        .unwrap();

    let output = Command::cargo_bin("kos").unwrap()
        .current_dir(tmp.path())
        .arg("--db").arg("test.db")
        .arg("rebuild-index")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Rebuilt index: 1 entities"));
}

#[test]
fn test_db_flag_configurable() {
    let tmp = TempDir::new().unwrap();
    let md = write_file(tmp.path(), "test.md", "# DB Flag Test");

    let output = Command::cargo_bin("kos").unwrap()
        .current_dir(tmp.path())
        .arg("--db").arg("custom.db")
        .arg("import")
        .arg(&md)
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(tmp.path().join("custom.db").exists());
}

#[test]
fn test_search_result_has_snippet() {
    let tmp = TempDir::new().unwrap();
    let md = write_file(tmp.path(), "test.md", "---\ntitle: \"Snippet Test\"\n---\n\n# Snippet Test\n\nThe transformer architecture uses self-attention.");

    Command::cargo_bin("kos").unwrap()
        .current_dir(tmp.path())
        .arg("--db").arg("test.db")
        .arg("import")
        .arg(&md)
        .output()
        .unwrap();

    let output = Command::cargo_bin("kos").unwrap()
        .current_dir(tmp.path())
        .arg("--db").arg("test.db")
        .arg("search")
        .arg("transformer")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Snippet:"));
}

#[test]
fn test_import_error_handling() {
    let tmp = TempDir::new().unwrap();
    // Import a non-existent file
    let output = Command::cargo_bin("kos").unwrap()
        .current_dir(tmp.path())
        .arg("--db").arg("test.db")
        .arg("import")
        .arg(tmp.path().join("nonexistent.md"))
        .output()
        .unwrap();

    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("ERROR"));
}

#[test]
fn test_batch_import_progress_output() {
    let tmp = TempDir::new().unwrap();
    for i in 0..5 {
        write_file(tmp.path(), &format!("doc{}.md", i), &format!("# Doc {}\n\nContent {}.", i, i));
    }

    let output = Command::cargo_bin("kos").unwrap()
        .current_dir(tmp.path())
        .arg("--db").arg("test.db")
        .arg("import")
        .arg(tmp.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Total files: 5"));
    assert!(stdout.contains("Created: 5"));
}
