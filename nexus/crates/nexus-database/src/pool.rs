use crate::{DatabaseBackend, DatabaseError, Dialect, SqlValue, Transaction};
use async_trait::async_trait;
use serde_json::{json, Value};
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
        let tx = self
            .pool
            .begin()
            .await
            .map_err(|e| DatabaseError::Transaction(e.to_string()))?;
        Ok(Box::new(SqlxTransaction {
            inner: Some(tx),
        }))
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

        // Try to preserve native types instead of converting everything to String
        let value = row_column_to_value(row, col);
        map.insert(name, value);
    }

    map
}

/// Extract a column value preserving its native JSON type.
fn row_column_to_value(row: &sqlx::any::AnyRow, col: &<sqlx::Any as sqlx::Database>::Column) -> Value {
    use sqlx::{Column, Row, TypeInfo, ValueRef as _};

    let col_name = col.name();
    let type_name = col.type_info().name().to_uppercase();

    // Check for NULL first
    if let Ok(raw) = row.try_get_raw(col_name) {
        if raw.is_null() {
            return Value::Null;
        }
    }

    // Try typed extraction based on column type info
    match type_name.as_str() {
        "BOOL" | "BOOLEAN" => {
            if let Ok(v) = row.try_get::<bool, _>(col_name) {
                return Value::Bool(v);
            }
        }
        "INT2" | "INT4" | "INT8" | "SMALLINT" | "INTEGER" | "BIGINT" | "SERIAL"
        | "BIGSERIAL" | "TINYINT" | "MEDIUMINT" => {
            if let Ok(v) = row.try_get::<i64, _>(col_name) {
                return json!(v);
            }
            if let Ok(v) = row.try_get::<i32, _>(col_name) {
                return json!(v);
            }
        }
        "FLOAT4" | "FLOAT8" | "REAL" | "DOUBLE" | "DOUBLE PRECISION" | "NUMERIC" | "DECIMAL" => {
            if let Ok(v) = row.try_get::<f64, _>(col_name) {
                return json!(v);
            }
        }
        "JSON" | "JSONB" => {
            // Try as string first, then parse as JSON
            if let Ok(s) = row.try_get::<String, _>(col_name) {
                if let Ok(parsed) = serde_json::from_str::<Value>(&s) {
                    return parsed;
                }
                return Value::String(s);
            }
        }
        _ => {}
    }

    // Fallback: try as string
    if let Ok(v) = row.try_get::<String, _>(col_name) {
        return Value::String(v);
    }

    // Last resort: null
    Value::Null
}

// ── Transaction implementation ──────────────────────────────────

/// SQLx transaction wrapper implementing the Transaction trait.
struct SqlxTransaction {
    inner: Option<sqlx::Transaction<'static, sqlx::Any>>,
}

#[async_trait]
impl Transaction for SqlxTransaction {
    async fn execute(&mut self, sql: &str, bindings: &[SqlValue]) -> Result<u64, DatabaseError> {
        let tx = self
            .inner
            .as_mut()
            .ok_or_else(|| DatabaseError::Transaction("Transaction already consumed".into()))?;

        let mut query = sqlx::query(sql);
        for binding in bindings {
            query = bind_value(query, binding);
        }

        let result = query
            .execute(&mut **tx)
            .await
            .map_err(|e| DatabaseError::Query(e.to_string()))?;

        Ok(result.rows_affected())
    }

    async fn query(
        &mut self,
        sql: &str,
        bindings: &[SqlValue],
    ) -> Result<Vec<HashMap<String, Value>>, DatabaseError> {
        let tx = self
            .inner
            .as_mut()
            .ok_or_else(|| DatabaseError::Transaction("Transaction already consumed".into()))?;

        let mut query = sqlx::query(sql);
        for binding in bindings {
            query = bind_value(query, binding);
        }

        let rows = query
            .fetch_all(&mut **tx)
            .await
            .map_err(|e| DatabaseError::Query(e.to_string()))?;

        let mut results = Vec::with_capacity(rows.len());
        for row in &rows {
            results.push(row_to_map(row));
        }

        Ok(results)
    }

    async fn commit(mut self: Box<Self>) -> Result<(), DatabaseError> {
        let tx = self
            .inner
            .take()
            .ok_or_else(|| DatabaseError::Transaction("Transaction already consumed".into()))?;

        tx.commit()
            .await
            .map_err(|e| DatabaseError::Transaction(e.to_string()))
    }

    async fn rollback(mut self: Box<Self>) -> Result<(), DatabaseError> {
        let tx = self
            .inner
            .take()
            .ok_or_else(|| DatabaseError::Transaction("Transaction already consumed".into()))?;

        tx.rollback()
            .await
            .map_err(|e| DatabaseError::Transaction(e.to_string()))
    }
}
