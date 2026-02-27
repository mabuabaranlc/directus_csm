pub mod mongodb_backend;

// Future implementations:
// pub mod bigquery;
// pub mod synapse;

use async_trait::async_trait;
use nexus_types::query::Query;
use serde_json::Value;

pub type Item = Value;
pub type PrimaryKey = Value;

/// High-level data backend trait for NoSQL databases
/// Operates on items directly rather than SQL
#[async_trait]
pub trait DataBackend: Send + Sync {
    /// Insert items into a collection
    async fn create(
        &self,
        collection: &str,
        data: Vec<Item>,
    ) -> Result<Vec<PrimaryKey>, NoSqlError>;

    /// Read items from a collection with query parameters
    async fn read(
        &self,
        collection: &str,
        query: &Query,
    ) -> Result<Vec<Item>, NoSqlError>;

    /// Update items in a collection by primary keys
    async fn update(
        &self,
        collection: &str,
        keys: &[PrimaryKey],
        data: Item,
    ) -> Result<Vec<PrimaryKey>, NoSqlError>;

    /// Delete items from a collection by primary keys
    async fn delete(
        &self,
        collection: &str,
        keys: &[PrimaryKey],
    ) -> Result<Vec<PrimaryKey>, NoSqlError>;

    /// Introspect the database schema
    async fn introspect_schema(
        &self,
    ) -> Result<nexus_types::schema::SchemaOverview, NoSqlError>;

    /// Whether this backend supports transactions
    fn supports_transactions(&self) -> bool;

    /// Whether this backend supports relational queries
    fn supports_relations(&self) -> bool;
}

#[derive(Debug, thiserror::Error)]
pub enum NoSqlError {
    #[error("Connection error: {0}")]
    Connection(String),
    #[error("Query error: {0}")]
    Query(String),
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Schema error: {0}")]
    Schema(String),
    #[error("Unsupported operation: {0}")]
    Unsupported(String),
}
