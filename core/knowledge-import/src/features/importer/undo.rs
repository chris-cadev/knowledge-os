use knowledge_core::ports::import_record::ImportRecord;
use std::collections::HashMap;
use std::path::Path;
use std::sync::LazyLock;
use std::sync::Mutex;
use uuid::Uuid;

use super::adapter::ImportError;
use super::directory::compute_content_hash;

static IMPORT_HISTORY: LazyLock<Mutex<Vec<ImportRecord>>> =
    LazyLock::new(|| Mutex::new(Vec::new()));

static CONTENT_HASHES: LazyLock<Mutex<HashMap<String, String>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub fn record_import(
    source_path: &Path,
    entity_ids: Vec<Uuid>,
    format: &str,
) -> Result<ImportRecord, ImportError> {
    let content_hash = compute_content_hash(source_path).ok();
    let record = ImportRecord {
        id: Uuid::new_v4(),
        source_path: source_path.to_string_lossy().to_string(),
        entity_ids,
        imported_at: chrono::Utc::now(),
        format: format.to_string(),
        content_hash,
    };

    if let Some(ref hash) = record.content_hash {
        if let Ok(mut hashes) = CONTENT_HASHES.lock() {
            hashes.insert(record.source_path.clone(), hash.clone());
        }
    }

    if let Ok(mut history) = IMPORT_HISTORY.lock() {
        history.push(record.clone());
    }

    Ok(record)
}

pub fn undo_last_import() -> Result<Option<ImportRecord>, ImportError> {
    if let Ok(mut history) = IMPORT_HISTORY.lock() {
        Ok(history.pop())
    } else {
        Ok(None)
    }
}

pub fn get_import_history() -> Vec<ImportRecord> {
    if let Ok(history) = IMPORT_HISTORY.lock() {
        history.clone()
    } else {
        Vec::new()
    }
}

pub fn get_content_hashes() -> HashMap<String, String> {
    if let Ok(hashes) = CONTENT_HASHES.lock() {
        hashes.clone()
    } else {
        HashMap::new()
    }
}

pub fn get_previous_hashes() -> HashMap<String, String> {
    get_content_hashes()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_record_and_undo() {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(b"test content for record").unwrap();
        file.flush().unwrap();

        let entity_id = Uuid::new_v4();
        let record = record_import(file.path(), vec![entity_id], "test").unwrap();
        assert_eq!(record.entity_ids.len(), 1);
        assert_eq!(record.format, "test");

        let undone = undo_last_import().unwrap();
        assert!(undone.is_some());
        assert_eq!(undone.unwrap().entity_ids[0], entity_id);
    }

    #[test]
    fn test_conflict_detection() {
        let content = format!("conflict_test_{}", Uuid::new_v4());
        let mut f1 = NamedTempFile::new().unwrap();
        f1.write_all(content.as_bytes()).unwrap();
        f1.flush().unwrap();

        let content2 = content.clone();
        let mut f2 = NamedTempFile::new().unwrap();
        f2.write_all(content2.as_bytes()).unwrap();
        f2.flush().unwrap();

        let eid = Uuid::new_v4();
        record_import(f1.path(), vec![eid], "test").unwrap();

        let all_hashes = get_previous_hashes();
        let f2_hash = compute_content_hash(f2.path()).unwrap();
        let is_conflict = all_hashes.values().any(|h| h == &f2_hash);
        assert!(is_conflict);
    }
}
