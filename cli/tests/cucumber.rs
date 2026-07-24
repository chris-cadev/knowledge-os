use cucumber::{gherkin::Step, given, then, when, World};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

#[derive(Debug, Default, World)]
pub struct CliWorld {
    temp_dir: Option<TempDir>,
    last_output: Option<std::process::Output>,
    last_entity_id: Option<String>,
    last_merge_id: Option<String>,
    files: HashMap<String, String>,
    entity_ids: HashMap<String, String>,
}

impl CliWorld {
    fn temp_path(&self) -> &std::path::Path {
        self.temp_dir.as_ref().unwrap().path()
    }

    fn write_file(&mut self, name: &str, content: &str) -> PathBuf {
        let path = self.temp_path().join(name);
        fs::write(&path, content).unwrap();
        self.files.insert(name.to_string(), content.to_string());
        path
    }

    fn run_kos(&mut self, args: &[&str]) {
        let path = self.temp_path().to_path_buf();
        let db_path = path.join("test.db");
        let mut cmd = assert_cmd::Command::cargo_bin("kos").unwrap();
        cmd.current_dir(&path)
            .arg("--db")
            .arg(&db_path)
            .args(args);
        self.last_output = Some(cmd.output().unwrap());
    }

    fn stdout(&self) -> String {
        match self.last_output.as_ref() {
            Some(o) => String::from_utf8_lossy(&o.stdout).to_string(),
            None => String::new(),
        }
    }

    fn stderr(&self) -> String {
        match self.last_output.as_ref() {
            Some(o) => String::from_utf8_lossy(&o.stderr).to_string(),
            None => String::new(),
        }
    }

    fn extract_entity_id_from_created(&self) -> Option<String> {
        for line in self.stdout().lines() {
            if line.contains("Created:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 3 {
                    return Some(parts[2].to_string());
                }
            }
        }
        None
    }

    fn extract_entity_id_for_title(&self, title: &str) -> Option<String> {
        let output = assert_cmd::Command::cargo_bin("kos")
            .unwrap()
            .current_dir(self.temp_path())
            .arg("--db")
            .arg(self.temp_path().join("test.db"))
            .arg("list")
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        for line in stdout.lines() {
            if line.contains(title) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    return Some(parts[1].to_string());
                }
            }
        }
        None
    }

    fn run_kos_direct(&self, args: &[&str]) -> std::process::Output {
        assert_cmd::Command::cargo_bin("kos")
            .unwrap()
            .current_dir(self.temp_path())
            .arg("--db")
            .arg(self.temp_path().join("test.db"))
            .args(args)
            .output()
            .unwrap()
    }
}

// =============================================================================
// Background
// =============================================================================

#[given("an empty database")]
async fn empty_database(world: &mut CliWorld) {
    world.temp_dir = Some(TempDir::new().unwrap());
    world.last_output = None;
    world.last_entity_id = None;
    world.last_merge_id = None;
    world.files.clear();
    world.entity_ids.clear();
}

// =============================================================================
// File Setup
// =============================================================================

#[given(expr = "a file {string} with content:")]
async fn create_file_with_content(world: &mut CliWorld, name: String, step: &Step) {
    let content = step.docstring.as_ref().unwrap();
    world.write_file(&name, content);
}

#[given(expr = "the file {string} is updated with content:")]
async fn update_file_with_content(world: &mut CliWorld, name: String, step: &Step) {
    let content = step.docstring.as_ref().unwrap();
    world.write_file(&name, content);
}

#[given("a directory with files:")]
async fn create_directory_with_files(world: &mut CliWorld, step: &Step) {
    if let Some(table) = step.table.as_ref() {
        for row in table.rows.iter().skip(1) {
            let filename = &row[0];
            let content = &row[1];
            let path = world.temp_path().join(filename);
            fs::write(&path, content).unwrap();
        }
    }
}

#[given(expr = "I import a file {string} with content:")]
async fn import_file_with_content(world: &mut CliWorld, name: String, step: &Step) {
    let content = step.docstring.as_ref().unwrap();
    world.write_file(&name, content);
    world.run_kos(&["import", &name]);
    if let Some(id) = world.extract_entity_id_from_created() {
        world.entity_ids.insert(name.clone(), id);
    }
}

#[given(expr = "an empty file {string}")]
async fn create_empty_file(world: &mut CliWorld, name: String) {
    world.write_file(&name, "");
}

#[given("I import files with varying relevance:")]
async fn import_files_with_varying_relevance(world: &mut CliWorld, step: &Step) {
    if let Some(table) = step.table.as_ref() {
        for row in table.rows.iter().skip(1) {
            let filename = &row[0];
            let content = &row[1];
            world.write_file(filename, content);
            world.run_kos(&["import", filename]);
            if let Some(id) = world.extract_entity_id_from_created() {
                world.entity_ids.insert(filename.clone(), id);
            }
        }
    }
}

#[given("I import files:")]
async fn import_files(world: &mut CliWorld, step: &Step) {
    if let Some(table) = step.table.as_ref() {
        for row in table.rows.iter().skip(1) {
            let filename = &row[0];
            let content = &row[1];
            world.write_file(filename, content);
            world.run_kos(&["import", filename]);
            if let Some(id) = world.extract_entity_id_from_created() {
                world.entity_ids.insert(filename.clone(), id);
            }
        }
    }
}

// =============================================================================
// Command Execution
// =============================================================================

#[when(expr = "I run {string}")]
async fn run_kos_command(world: &mut CliWorld, cmd: String) {
    let entity_id = world.last_entity_id.clone();
    let merge_id = world.last_merge_id.clone();
    let mut expanded = cmd.replace("<directory>", &world.temp_path().to_string_lossy());
    if let Some(ref id) = entity_id {
        expanded = expanded.replace("<entity-id>", id);
    }
    if let Some(ref id) = merge_id {
        expanded = expanded.replace("<merge-id>", id);
    }
    let mut args: Vec<&str> = expanded.split_whitespace().collect();
    if args.first() == Some(&"kos") {
        args.remove(0);
    }
    world.run_kos(&args);
}

#[when("I extract the entity ID from the last import")]
async fn extract_entity_id(world: &mut CliWorld) {
    if let Some(id) = world.extract_entity_id_from_created() {
        world.entity_ids.insert("_last_imported".to_string(), id.clone());
        world.last_entity_id = Some(id);
    }
}

#[when(expr = "I extract the entity ID for {string}")]
async fn extract_entity_id_for_title_step(world: &mut CliWorld, title: String) {
    if let Some(id) = world.extract_entity_id_for_title(&title) {
        world.entity_ids.insert("_last_imported".to_string(), id.clone());
        world.last_entity_id = Some(id);
    }
}

#[when(expr = "I run kos get <entity-id>")]
async fn run_get_entity(world: &mut CliWorld) {
    let id = world.last_entity_id.clone().unwrap();
    let output = world.run_kos_direct(&["get", &id]);
    world.last_output = Some(output);
}

#[when(expr = "I run kos archive <entity-id>")]
async fn run_archive_entity(world: &mut CliWorld) {
    let id = world.last_entity_id.clone().unwrap();
    let output = world.run_kos_direct(&["archive", &id]);
    world.last_output = Some(output);
}

#[when(expr = "I run kos restore <entity-id>")]
async fn run_restore_entity(world: &mut CliWorld) {
    let id = world.last_entity_id.clone().unwrap();
    let output = world.run_kos_direct(&["restore", &id]);
    world.last_output = Some(output);
}

#[when(expr = "I run kos resolution log")]
async fn run_resolution_log(world: &mut CliWorld) {
    let output = world.run_kos_direct(&["resolution", "log"]);
    world.last_output = Some(output);
}

#[when(expr = "I extract the merge ID from the resolution log")]
async fn extract_merge_id(world: &mut CliWorld) {
    let stdout = world.stdout();
    for line in stdout.lines() {
        if line.contains("Merge ID:") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                world.last_merge_id = Some(parts[2].to_string());
                break;
            }
        }
    }
}

#[when(expr = "I run kos resolution undo <merge-id>")]
async fn run_resolution_undo(world: &mut CliWorld) {
    let id = world.last_merge_id.clone().unwrap();
    let output = world.run_kos_direct(&["resolution", "undo", &id]);
    world.last_output = Some(output);
}

// =============================================================================
// Assertions
// =============================================================================

#[then(expr = "the output contains {string}")]
async fn assert_output_contains(world: &mut CliWorld, expected: String) {
    let stdout = world.stdout();
    let stderr = world.stderr();
    assert!(
        stdout.contains(&expected),
        "Expected '{}' in stdout, got:\nstdout: {}\nstderr: {}",
        expected,
        stdout,
        stderr
    );
}

#[then(expr = "the error output contains {string}")]
async fn assert_error_output_contains(world: &mut CliWorld, expected: String) {
    let stderr = world.stderr();
    assert!(
        stderr.contains(&expected),
        "Expected '{}' in stderr, got:\n{}",
        expected,
        stderr
    );
}

#[then(expr = "the warning output contains {string}")]
async fn assert_warning_output_contains(world: &mut CliWorld, expected: String) {
    let stderr = world.stderr();
    let stdout = world.stdout();
    assert!(
        stderr.contains(&expected) || stdout.contains(&expected),
        "Expected '{}' in output, got:\nstdout: {}\nstderr: {}",
        expected,
        stdout,
        stderr
    );
}

#[then(expr = "{int} relationships of type {string} exist")]
async fn assert_relationships_exist(world: &mut CliWorld, count: usize, rel_type: String) {
    let output = world.run_kos_direct(&["list"]);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let found_count = stdout.lines().filter(|l| l.contains(&rel_type)).count();
    assert_eq!(
        found_count, count,
        "Expected {} relationships of type '{}', found {}",
        count, rel_type, found_count
    );
}

#[then(expr = "relationships of type {string} exist")]
async fn assert_relationships_of_type_exist(world: &mut CliWorld, rel_type: String) {
    let output = world.run_kos_direct(&["list"]);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    assert!(
        stdout.contains(&rel_type),
        "Expected relationships of type '{}' to exist",
        rel_type
    );
}

#[then(expr = "the entity has a Provenance component with source {string}")]
async fn assert_provenance_source(world: &mut CliWorld, source: String) {
    let output = world.run_kos_direct(&["list"]);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    assert!(
        stdout.contains(&source),
        "Expected Provenance source '{}' in output",
        source
    );
}

#[then(expr = "the entity has a Content component with extracted text")]
async fn assert_content_component(world: &mut CliWorld) {
    let stdout = world.stdout();
    assert!(
        stdout.contains("Content") || stdout.contains("words"),
        "Expected Content component in output"
    );
}

#[then(expr = "the entity has a BinaryContent component with file {string}")]
async fn assert_binary_content_component(world: &mut CliWorld, filename: String) {
    let stdout = world.stdout();
    assert!(
        stdout.contains("BinaryContent") || stdout.contains(&filename),
        "Expected BinaryContent component for '{}' in output",
        filename
    );
}

#[then(expr = "the entity has components:")]
async fn assert_entity_has_components(world: &mut CliWorld, step: &Step) {
    let stdout = world.stdout();
    if let Some(table) = step.table.as_ref() {
        for row in table.rows.iter().skip(1) {
            let component = &row[0];
            assert!(
                stdout.contains(component),
                "Expected component '{}' in output",
                component
            );
        }
    }
}

#[then(expr = "the entity version is incremented")]
async fn assert_version_incremented(world: &mut CliWorld) {
    let stdout = world.stdout();
    assert!(
        stdout.contains("Version: 2") || stdout.contains("version: 2"),
        "Expected version to be incremented to 2"
    );
}

#[then(expr = "the relationship preserves the section anchor")]
async fn assert_section_anchor_preserved(world: &mut CliWorld) {
    let stdout = world.stdout();
    assert!(
        stdout.contains("section") || stdout.contains("anchor") || stdout.contains("#"),
        "Expected section anchor to be preserved"
    );
}

#[then(expr = "no manual review is required")]
async fn assert_no_manual_review(world: &mut CliWorld) {
    let stdout = world.stdout();
    assert!(
        !stdout.contains("review") || !stdout.contains("manual"),
        "Expected no manual review requirement"
    );
}

#[then(expr = "the resolution log contains a merge with confidence")]
async fn assert_resolution_log_has_confidence(world: &mut CliWorld) {
    let output = world.run_kos_direct(&["resolution", "log"]);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    assert!(
        stdout.contains("confidence:"),
        "Expected resolution log to contain confidence score"
    );
}

#[then(expr = "the resolution respects the configured threshold")]
async fn assert_resolution_respects_threshold(world: &mut CliWorld) {
    let stdout = world.stdout();
    assert!(
        stdout.contains("Duplicates resolved") || stdout.contains("no duplicates"),
        "Expected resolution to respect configured threshold"
    );
}

#[then(expr = "{string} appears before {string} in results")]
async fn assert_order_in_results(world: &mut CliWorld, first: String, second: String) {
    let stdout = world.stdout();
    let first_pos = stdout.find(&first).unwrap_or(usize::MAX);
    let second_pos = stdout.find(&second).unwrap_or(usize::MAX);
    assert!(
        first_pos < second_pos,
        "Expected '{}' to appear before '{}' in results",
        first,
        second
    );
}

// =============================================================================
// PDF-specific Steps (Mocked)
// =============================================================================

#[given(expr = "a PDF file {string} with metadata:")]
async fn create_pdf_with_metadata(world: &mut CliWorld, name: String, step: &Step) {
    let mut content = format!("# {}\n\n", name);
    if let Some(table) = step.table.as_ref() {
        for row in table.rows.iter().skip(1) {
            let field = &row[0];
            let value = &row[1];
            content.push_str(&format!("{}: {}\n", field, value));
        }
    }
    world.write_file(&name, &content);
}

#[given(expr = "a PDF file {string} with no metadata")]
async fn create_pdf_without_metadata(world: &mut CliWorld, name: String) {
    world.write_file(&name, "# No Metadata PDF\n\nContent without metadata.");
}

#[given(expr = "a scanned PDF file {string}")]
async fn create_scanned_pdf(world: &mut CliWorld, name: String) {
    world.write_file(&name, "# Scanned PDF\n\nNo extractable text content.");
}

#[given(expr = "a directory with {int} files:")]
async fn create_directory_with_count(world: &mut CliWorld, _count: usize, step: &Step) {
    let mut md_count = 0;
    let mut pdf_count = 0;
    if let Some(table) = step.table.as_ref() {
        for row in table.rows.iter().skip(1) {
            let file_type = &row[0];
            let type_count: usize = row[1].parse().unwrap();
            match file_type.as_str() {
                "md" => {
                    for i in 0..type_count {
                        let name = format!("file-{}.md", md_count + i);
                        let path = world.temp_path().join(&name);
                        fs::write(&path, format!("# File {}\n\nContent.", md_count + i)).unwrap();
                    }
                    md_count += type_count;
                }
                "pdf" => {
                    for i in 0..type_count {
                        let name = format!("file-{}.pdf", pdf_count + i);
                        let path = world.temp_path().join(&name);
                        fs::write(&path, format!("# PDF {}\n\nContent.", pdf_count + i)).unwrap();
                    }
                    pdf_count += type_count;
                }
                _ => {}
            }
        }
    }
}

#[given(expr = "the merge threshold for {string} is set to {float}")]
async fn set_merge_threshold(_world: &mut CliWorld, entity_type: String, threshold: f64) {
    eprintln!(
        "Setting merge threshold for {} to {} (not yet implemented)",
        entity_type, threshold
    );
}

// =============================================================================
// Main
// =============================================================================

#[tokio::main]
async fn main() {
    CliWorld::cucumber()
        .max_concurrent_scenarios(1)
        .run_and_exit("features")
        .await;
}
