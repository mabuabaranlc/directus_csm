pub mod fetch_permissions;
pub mod fetch_policies;
pub mod fetch_roles_tree;
pub mod process_ast;
pub mod validate_access;

use nexus_cache::CacheStore;
use nexus_database::DatabaseBackend;
use nexus_types::schema::SchemaOverview;
use std::sync::Arc;

/// Permission engine context — shared across all permission modules
pub struct PermissionContext {
    pub db: Arc<dyn DatabaseBackend>,
    pub schema: Arc<SchemaOverview>,
    pub cache: Option<Arc<dyn CacheStore>>,
}
