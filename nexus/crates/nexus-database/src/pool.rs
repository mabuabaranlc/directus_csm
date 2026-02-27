use crate::{DatabaseBackend, DatabaseError, Dialect, SqlValue, Transaction};
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;

/// Multi-database connection pool using SQLx
pub struct DatabasePool {
    dialect: Dialect,
    pool: sqlx::AnyPool,
}

impl DatabasePool {
    pub async fn new(database_url: &str) -> Result<Self, DatabaseError> {
        let dialect = detect_dialect(database_url)?;

        let pool = sqlx::any::AnyPoolOptions::new()
            .max_connections(10)
            .connect(database_url)
            .await
            .map_err(|e| DatabaseError::Connection(e.to_string()))?;

        Ok(Self { dialect, pool })
    }

    pub fn pool(&self) -> &sqlx::AnyPool {
        &self.pool
    }
}

#[async_trait]
impl DatabaseBackend for DatabasePool {
    async fn execute(&self, sql: &str, bindings: &[SqlValue]) -> Result<u64, DatabaseError> {
        let mut query = sqlx::query(sql);

        for binding in bindings {
            query = bind_value(query, binding);
        }

        let result = query
            .execute(&self.pool)
            .await
            .map_err(|e| DatabaseError::Query(e.to_string()))?;

        Ok(result.rows_affected())
    }

    async fn query(
        &self,
        sql: &str,
        bindings: &[SqlValue],
    ) -> Result<Vec<HashMap<String, Value>>, DatabaseError> {
        let mut query = sqlx::query(sql);

        for binding in bindings {
            query = bind_value(query, binding);
        }

        let rows = query
            .fetch_all(&self.pool)
            .await
            .map_err(|e| DatabaseError::Query(e.to_string()))?;

        let mut results = Vec::with_capacity(rows.len());
        for row in &rows {
            results.push(row_to_map(row));
        }

        Ok(results)
    }

    async fn begin_transaction(&self) -> Result<Box<dyn Transaction>, DatabaseError> {
        // SQLx transaction support will be implemented here
        Err(DatabaseError::NotSupported(
            "Transactions not yet implemented for AnyPool".to_string(),
        ))
    }

    fn dialect(&self) -> Dialect {
        self.dialect
    }
}

fn detect_dialect(url: &str) -> Result<Dialect, DatabaseError> {
    if url.starts_with("postgres://") || url.starts_with("postgresql://") {
        Ok(Dialect::Postgres)
    } else if url.starts_with("mysql://") {
        Ok(Dialect::MySQL)
    } else if url.starts_with("sqlite://") || url.starts_with("sqlite:") {
        Ok(Dialect::SQLite)
    } else if url.starts_with("mssql://") {
        Ok(Dialect::MSSQL)
    } else {
        Err(DatabaseError::Connection(format!(
            "Unsupported database URL scheme: {}",
            url.split("://").next().unwrap_or("unknown")
        )))
    }
}

fn bind_value<'q>(
    query: sqlx::query::Query<'q, sqlx::Any, sqlx::any::AnyArguments<'q>>,
    value: &'q SqlValue,
) -> sqlx::query::Query<'q, sqlx::Any, sqlx::any::AnyArguments<'q>> {
    
    match value {
        SqlValue::Null => query,
        SqlValue::Bool(v) => query.bind(*v),
        SqlValue::Int(v) => query.bind(*v),
        SqlValue::Float(v) => query.bind(*v),
        SqlValue::Text(v) => query.bind(v.as_str()),
        SqlValue::Json(v) => query.bind(v.to_string()),
        SqlValue::Bytes(v) => query.bind(v.as_slice()),
    }
}

fn row_to_map(row: &sqlx::any::AnyRow) -> HashMap<String, Value> {
    use sqlx::{Column, Row};
    let mut map = HashMap::new();

    for col in row.columns() {
        let name = col.name().to_string();
        let value: Option<String> = row.try_get(col.name()).ok();
        map.insert(
            name,
            value
                .map(Value::String)
                .unwrap_or(Value::Null),
        );
    }

    map
}
