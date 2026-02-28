pub mod ast;
pub mod helpers;
pub mod inspector;
pub mod migrations;
pub mod pool;
pub mod seeds;

use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;

/// SQL dialect
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Dialect {
    Postgres,
    MySQL,
    SQLite,
    MSSQL,
    Oracle,
    CockroachDB,
}

impl std::fmt::Display for Dialect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Dialect::Postgres => write!(f, "postgres"),
            Dialect::MySQL => write!(f, "mysql"),
            Dialect::SQLite => write!(f, "sqlite"),
            Dialect::MSSQL => write!(f, "mssql"),
            Dialect::Oracle => write!(f, "oracle"),
            Dialect::CockroachDB => write!(f, "cockroachdb"),
        }
    }
}

/// SQL value for binding parameters
#[derive(Debug, Clone)]
pub enum SqlValue {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Text(String),
    Json(Value),
    Bytes(Vec<u8>),
}

/// Database backend trait for SQL databases
#[async_trait]
pub trait DatabaseBackend: Send + Sync {
    /// Execute a raw SQL statement (INSERT, UPDATE, DELETE)
    async fn execute(&self, sql: &str, bindings: &[SqlValue]) -> Result<u64, DatabaseError>;

    /// Query and return rows as JSON maps
    async fn query(
        &self,
        sql: &str,
        bindings: &[SqlValue],
    ) -> Result<Vec<HashMap<String, Value>>, DatabaseError>;

    /// Begin a transaction
    async fn begin_transaction(&self) -> Result<Box<dyn Transaction>, DatabaseError>;

    /// Get the dialect
    fn dialect(&self) -> Dialect;

    /// Quote an identifier for this dialect
    fn quote_identifier(&self, name: &str) -> String {
        match self.dialect() {
            Dialect::MySQL => format!("`{}`", name),
            Dialect::MSSQL => format!("[{}]", name),
            _ => format!("\"{}\"", name),
        }
    }
}

/// Transaction handle
#[async_trait]
pub trait Transaction: Send {
    async fn execute(&mut self, sql: &str, bindings: &[SqlValue]) -> Result<u64, DatabaseError>;
    async fn query(
        &mut self,
        sql: &str,
        bindings: &[SqlValue],
    ) -> Result<Vec<HashMap<String, Value>>, DatabaseError>;
    async fn commit(self: Box<Self>) -> Result<(), DatabaseError>;
    async fn rollback(self: Box<Self>) -> Result<(), DatabaseError>;
}

#[derive(Debug, thiserror::Error)]
pub enum DatabaseError {
    #[error("Connection error: {0}")]
    Connection(String),
    #[error("Query error: {0}")]
    Query(String),
    #[error("Migration error: {0}")]
    Migration(String),
    #[error("Schema error: {0}")]
    Schema(String),
    #[error("Transaction error: {0}")]
    Transaction(String),
    #[error("Not supported: {0}")]
    NotSupported(String),
}
