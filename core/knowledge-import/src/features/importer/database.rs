use async_trait::async_trait;
use knowledge_core::ports::database::{
    ColumnSchema, ConnectionInfo, DatabaseError, DatabaseSource, DbColumnValue, TableInfo,
    TablePreview,
};
use sqlx::Row;
use std::path::{Path, PathBuf};

pub struct SqliteDatabaseSource {
    pub path: PathBuf,
}

impl SqliteDatabaseSource {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

fn sqlite_connect_string(path: &Path) -> String {
    format!("sqlite:///{}?mode=rwc", path.display())
}

#[async_trait]
impl DatabaseSource for SqliteDatabaseSource {
    async fn test_connection(&self) -> Result<ConnectionInfo, DatabaseError> {
        let start = std::time::Instant::now();
        let pool = sqlx::SqlitePool::connect(&sqlite_connect_string(&self.path))
            .await
            .map_err(|e| DatabaseError::Connection(e.to_string()))?;
        let version: String = sqlx::query_scalar("SELECT sqlite_version()")
            .fetch_one(&pool)
            .await
            .map_err(|e| DatabaseError::Query(e.to_string()))?;
        let latency_ms = start.elapsed().as_millis() as u32;
        pool.close().await;
        Ok(ConnectionInfo {
            server_version: version,
            database_name: self
                .path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string(),
            reachable: true,
            latency_ms,
        })
    }

    async fn list_tables(&self) -> Result<Vec<TableInfo>, DatabaseError> {
        let pool = sqlx::SqlitePool::connect(&sqlite_connect_string(&self.path))
            .await
            .map_err(|e| DatabaseError::Connection(e.to_string()))?;

        let table_rows: Vec<(String,)> =
            sqlx::query_as("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
                .fetch_all(&pool)
                .await
                .map_err(|e| DatabaseError::Query(e.to_string()))?;

        let mut tables = Vec::new();
        for (name,) in table_rows {
            let col_rows: Vec<(String, String, bool)> =
                sqlx::query_as("SELECT name, type, \"notnull\" FROM pragma_table_info(?)")
                    .bind(&name)
                    .fetch_all(&pool)
                    .await
                    .map_err(|e| DatabaseError::Query(e.to_string()))?;

            let columns: Vec<ColumnSchema> = col_rows
                .into_iter()
                .map(|(col_name, col_type, not_null)| ColumnSchema {
                    name: col_name,
                    data_type: col_type,
                    nullable: !not_null,
                })
                .collect();

            let (count,): (i64,) = sqlx::query_as(&format!("SELECT COUNT(*) FROM \"{}\"", name))
                .fetch_one(&pool)
                .await
                .map_err(|e| DatabaseError::Query(e.to_string()))?;

            tables.push(TableInfo {
                name,
                columns,
                row_count: count as u64,
            });
        }

        pool.close().await;
        Ok(tables)
    }

    async fn preview_table(
        &self,
        table: &str,
        limit: usize,
    ) -> Result<TablePreview, DatabaseError> {
        let pool = sqlx::SqlitePool::connect(&sqlite_connect_string(&self.path))
            .await
            .map_err(|e| DatabaseError::Connection(e.to_string()))?;

        let (total,): (i64,) = sqlx::query_as(&format!("SELECT COUNT(*) FROM \"{}\"", table))
            .fetch_one(&pool)
            .await
            .map_err(|e| DatabaseError::Query(e.to_string()))?;

        let col_rows: Vec<(String, String, bool)> =
            sqlx::query_as("SELECT name, type, \"notnull\" FROM pragma_table_info(?)")
                .bind(table)
                .fetch_all(&pool)
                .await
                .map_err(|e| DatabaseError::Query(e.to_string()))?;

        let columns: Vec<ColumnSchema> = col_rows
            .into_iter()
            .map(|(name, data_type, not_null)| ColumnSchema {
                name,
                data_type,
                nullable: !not_null,
            })
            .collect();

        let rows = sqlx::query(&format!("SELECT * FROM \"{}\" LIMIT ?", table))
            .bind(limit as i64)
            .fetch_all(&pool)
            .await
            .map_err(|e| DatabaseError::Query(e.to_string()))?;

        let mut preview_rows = Vec::new();
        for row in rows {
            let mut row_values = Vec::new();
            for (i, _col) in columns.iter().enumerate() {
                let val = row.try_get::<String, usize>(i).ok();
                row_values.push(match val {
                    Some(s) => DbColumnValue::Text(s),
                    None => DbColumnValue::Null,
                });
            }
            preview_rows.push(row_values);
        }

        pool.close().await;
        Ok(TablePreview {
            columns,
            rows: preview_rows,
            total_rows: total as u64,
        })
    }
}

pub struct PostgresDatabaseSource {
    pub connection_string: String,
}

impl PostgresDatabaseSource {
    pub fn new(connection_string: String) -> Self {
        Self { connection_string }
    }
}

#[async_trait]
impl DatabaseSource for PostgresDatabaseSource {
    async fn test_connection(&self) -> Result<ConnectionInfo, DatabaseError> {
        let start = std::time::Instant::now();
        let pool = sqlx::PgPool::connect(&self.connection_string)
            .await
            .map_err(|e| DatabaseError::Connection(e.to_string()))?;
        let version: String = sqlx::query_scalar("SELECT version()")
            .fetch_one(&pool)
            .await
            .map_err(|e| DatabaseError::Query(e.to_string()))?;
        let latency_ms = start.elapsed().as_millis() as u32;
        pool.close().await;
        Ok(ConnectionInfo {
            server_version: version,
            database_name: "postgres".to_string(),
            reachable: true,
            latency_ms,
        })
    }

    async fn list_tables(&self) -> Result<Vec<TableInfo>, DatabaseError> {
        Err(DatabaseError::NotFound(
            "PostgreSQL list_tables not yet implemented".into(),
        ))
    }

    async fn preview_table(
        &self,
        _table: &str,
        _limit: usize,
    ) -> Result<TablePreview, DatabaseError> {
        Err(DatabaseError::NotFound(
            "PostgreSQL preview_table not yet implemented".into(),
        ))
    }
}

pub struct MySqlDatabaseSource {
    pub connection_string: String,
}

impl MySqlDatabaseSource {
    pub fn new(connection_string: String) -> Self {
        Self { connection_string }
    }
}

#[async_trait]
impl DatabaseSource for MySqlDatabaseSource {
    async fn test_connection(&self) -> Result<ConnectionInfo, DatabaseError> {
        let start = std::time::Instant::now();
        let pool = sqlx::MySqlPool::connect(&self.connection_string)
            .await
            .map_err(|e| DatabaseError::Connection(e.to_string()))?;
        let version: String = sqlx::query_scalar("SELECT version()")
            .fetch_one(&pool)
            .await
            .map_err(|e| DatabaseError::Query(e.to_string()))?;
        let latency_ms = start.elapsed().as_millis() as u32;
        pool.close().await;
        Ok(ConnectionInfo {
            server_version: version,
            database_name: "mysql".to_string(),
            reachable: true,
            latency_ms,
        })
    }

    async fn list_tables(&self) -> Result<Vec<TableInfo>, DatabaseError> {
        Err(DatabaseError::NotFound(
            "MySQL list_tables not yet implemented".into(),
        ))
    }

    async fn preview_table(
        &self,
        _table: &str,
        _limit: usize,
    ) -> Result<TablePreview, DatabaseError> {
        Err(DatabaseError::NotFound(
            "MySQL preview_table not yet implemented".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn db_path(name: &str) -> PathBuf {
        let dir = std::env::temp_dir();
        dir.join(format!("kos_test_{}.db", name))
    }

    fn sqlite_uri(path: &Path) -> String {
        format!("sqlite:///{}?mode=rwc", path.display())
    }

    fn cleanup(path: &Path) {
        let _ = std::fs::remove_file(path);
    }

    async fn create_populated_db(path: &Path) {
        let db_path = sqlite_uri(path);
        let pool = sqlx::SqlitePool::connect(&db_path).await.unwrap();
        sqlx::query("CREATE TABLE test (id INTEGER PRIMARY KEY, name TEXT)")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO test VALUES (1, 'hello'), (2, 'world')")
            .execute(&pool)
            .await
            .unwrap();
        pool.close().await;
    }

    #[tokio::test]
    async fn test_sqlite_test_connection_succeeds() {
        let path = db_path("test_connection");
        create_populated_db(&path).await;
        let source = SqliteDatabaseSource::new(path.clone());
        let info = source.test_connection().await.unwrap();
        assert!(info.reachable);
        assert!(!info.server_version.is_empty());
        cleanup(&path);
    }

    #[tokio::test]
    async fn test_sqlite_list_tables_empty() {
        let path = db_path("list_empty");
        // Create empty database file
        let pool = sqlx::SqlitePool::connect(&sqlite_uri(&path)).await.unwrap();
        pool.close().await;
        let source = SqliteDatabaseSource::new(path.clone());
        let tables = source.list_tables().await.unwrap();
        assert!(tables.is_empty());
        cleanup(&path);
    }

    #[tokio::test]
    async fn test_sqlite_list_tables_populated() {
        let path = db_path("list_populated");
        create_populated_db(&path).await;
        let source = SqliteDatabaseSource::new(path.clone());
        let tables = source.list_tables().await.unwrap();
        assert!(!tables.is_empty());
        let test_table = tables.iter().find(|t| t.name == "test").unwrap();
        assert_eq!(test_table.columns.len(), 2);
        cleanup(&path);
    }

    #[tokio::test]
    async fn test_sqlite_preview_table_returns_columns_and_sample() {
        let path = db_path("preview");
        create_populated_db(&path).await;
        let source = SqliteDatabaseSource::new(path.clone());
        let preview = source.preview_table("test", 10).await.unwrap();
        assert_eq!(preview.columns.len(), 2);
        assert_eq!(preview.total_rows, 2);
        cleanup(&path);
    }

    #[tokio::test]
    async fn test_postgres_connection_failure() {
        let source = PostgresDatabaseSource::new("postgres://invalid:5432/nonexistent".to_string());
        let result = source.test_connection().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_mysql_connection_failure() {
        let source = MySqlDatabaseSource::new("mysql://invalid:3306/nonexistent".to_string());
        let result = source.test_connection().await;
        assert!(result.is_err());
    }
}
