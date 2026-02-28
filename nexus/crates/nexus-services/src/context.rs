use nexus_bus::MessageBus;
use nexus_cache::CacheStore;
use nexus_database::DatabaseBackend;
use nexus_emitter::Emitter;
use nexus_types::accountability::Accountability;
use nexus_types::schema::SchemaOverview;
use std::sync::Arc;

/// Shared service context — passed to all service constructors
/// Mirrors the "options" pattern from Directus services
#[derive(Clone)]
pub struct ServiceContext {
    pub db: Arc<dyn DatabaseBackend>,
    pub schema: Arc<SchemaOverview>,
    pub accountability: Option<Accountability>,
    pub cache: Option<Arc<dyn CacheStore>>,
    pub emitter: Arc<Emitter>,
    pub bus: Option<Arc<dyn MessageBus>>,
}

impl std::fmt::Debug for ServiceContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ServiceContext")
            .field("accountability", &self.accountability)
            .finish_non_exhaustive()
    }
}

impl ServiceContext {
    pub fn new(
        db: Arc<dyn DatabaseBackend>,
        schema: Arc<SchemaOverview>,
        accountability: Option<Accountability>,
        cache: Option<Arc<dyn CacheStore>>,
        emitter: Arc<Emitter>,
    ) -> Self {
        Self {
            db,
            schema,
            accountability,
            cache,
            emitter,
            bus: None,
        }
    }

    /// Create context with a message bus for WebSocket event delivery
    pub fn with_bus(mut self, bus: Arc<dyn MessageBus>) -> Self {
        self.bus = Some(bus);
        self
    }

    /// Fork the context with a different accountability (e.g., for internal unauthenticated operations)
    pub fn fork_unauth(&self) -> Self {
        Self {
            db: self.db.clone(),
            schema: self.schema.clone(),
            accountability: None,
            cache: self.cache.clone(),
            emitter: self.emitter.clone(),
            bus: self.bus.clone(),
        }
    }

    /// Fork the context with specific accountability
    pub fn fork_with_accountability(&self, accountability: Accountability) -> Self {
        Self {
            db: self.db.clone(),
            schema: self.schema.clone(),
            accountability: Some(accountability),
            cache: self.cache.clone(),
            emitter: self.emitter.clone(),
            bus: self.bus.clone(),
        }
    }

    /// Check if the current user is an admin
    pub fn is_admin(&self) -> bool {
        self.accountability
            .as_ref()
            .map(|a| a.admin)
            .unwrap_or(false)
    }

    /// Get the current user ID
    pub fn user_id(&self) -> Option<&str> {
        self.accountability
            .as_ref()
            .and_then(|a| a.user.as_deref())
    }

    /// Get the current role ID
    pub fn role_id(&self) -> Option<&str> {
        self.accountability
            .as_ref()
            .and_then(|a| a.role.as_deref())
    }
}
