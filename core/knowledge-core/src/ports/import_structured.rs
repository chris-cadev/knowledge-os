use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnMapping {
    pub field_mappings: HashMap<String, FieldMapping>,
    pub skip_columns: HashSet<String>,
    pub entity_type_override: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FieldMapping {
    Title,
    Description,
    Content,
    Tags { separator: String },
    CustomComponent { component_name: String },
    TimelineDate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportPreview {
    pub columns: Vec<ColumnInfo>,
    pub sample_rows: Vec<Vec<ColumnValue>>,
    pub row_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnInfo {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ColumnValue {
    Text(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Null,
}
