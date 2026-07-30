use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionInfo {
    pub server_version: String,
    pub database_name: String,
    pub reachable: bool,
    pub latency_ms: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableInfo {
    pub name: String,
    pub columns: Vec<ColumnSchema>,
    pub row_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnSchema {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TablePreview {
    pub columns: Vec<ColumnSchema>,
    pub rows: Vec<Vec<DbColumnValue>>,
    pub total_rows: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DbColumnValue {
    Text(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Null,
}

impl fmt::Display for DbColumnValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DbColumnValue::Text(s) => write!(f, "{}", s),
            DbColumnValue::Integer(i) => write!(f, "{}", i),
            DbColumnValue::Float(fl) => write!(f, "{}", fl),
            DbColumnValue::Boolean(b) => write!(f, "{}", b),
            DbColumnValue::Null => write!(f, "NULL"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DatabaseError {
    Connection(String),
    Query(String),
    NotFound(String),
}

impl fmt::Display for DatabaseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DatabaseError::Connection(msg) => write!(f, "connection error: {}", msg),
            DatabaseError::Query(msg) => write!(f, "query error: {}", msg),
            DatabaseError::NotFound(msg) => write!(f, "not found: {}", msg),
        }
    }
}

impl std::error::Error for DatabaseError {}

#[async_trait]
pub trait DatabaseSource: Send + Sync {
    async fn test_connection(&self) -> Result<ConnectionInfo, DatabaseError>;
    async fn list_tables(&self) -> Result<Vec<TableInfo>, DatabaseError>;
    async fn preview_table(&self, table: &str, limit: usize)
        -> Result<TablePreview, DatabaseError>;
}
