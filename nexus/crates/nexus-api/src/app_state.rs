use nexus_bus::MessageBus;
use nexus_cache::CacheStore;
use nexus_database::DatabaseBackend;
use nexus_emitter::Emitter;
use nexus_services::context::ServiceContext;
use nexus_types::accountability::Accountability;
use nexus_types::schema::SchemaOverview;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Shared application state injected into all request handlers
pub struct AppState {
    pub db: Arc<dyn DatabaseBackend>,
    pub schema: Arc<RwLock<Arc<SchemaOverview>>>,
    pub cache: Option<Arc<dyn CacheStore>>,
    pub emitter: Arc<Emitter>,
    pub bus: Option<Arc<dyn MessageBus>>,
}

impl AppState {
    pub fn new(
        db: Arc<dyn DatabaseBackend>,
        schema: SchemaOverview,
        cache: Option<Arc<dyn CacheStore>>,
        emitter: Emitter,
    ) -> Self {
        Self {
            db,
            schema: Arc::new(RwLock::new(Arc::new(schema))),
            cache,
            emitter: Arc::new(emitter),
            bus: None,
        }
    }

    /// Set the message bus for WebSocket event delivery
    pub fn with_bus(mut self, bus: Arc<dyn MessageBus>) -> Self {
        self.bus = Some(bus);
        self
    }

    /// Create a ServiceContext from the app state for a given request
    pub async fn service_context(
        &self,
        accountability: Option<Accountability>,
    ) -> ServiceContext {
        let schema = self.schema.read().await.clone();
        let mut ctx = ServiceContext::new(
            self.db.clone(),
            schema,
            accountability,
            self.cache.clone(),
            self.emitter.clone(),
        );
        if let Some(ref bus) = self.bus {
            ctx = ctx.with_bus(bus.clone());
        }
        ctx
    }
}
