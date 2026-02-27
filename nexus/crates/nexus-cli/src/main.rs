use clap::{Parser, Subcommand};
use nexus_api::app_state::AppState;
use nexus_api::middleware::authenticate::Authentication;
use nexus_api::middleware::security_headers::SecurityHeaders;
use nexus_database::pool::DatabasePool;
use nexus_emitter::Emitter;
use nexus_websocket::WebSocketManager;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Parser)]
#[command(name = "nexus")]
#[command(about = "Nexus CMS — A high-performance headless CMS built in Rust")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the Nexus server
    Start {
        /// Host to bind to
        #[arg(long, default_value = "0.0.0.0")]
        host: String,
        /// Port to listen on
        #[arg(long, default_value = "8055")]
        port: u16,
    },
    /// Initialize database and create admin user
    Bootstrap {
        /// Admin email
        #[arg(long)]
        admin_email: Option<String>,
        /// Admin password
        #[arg(long)]
        admin_password: Option<String>,
    },
    /// Run pending database migrations
    Migrate,
    /// Schema management commands
    Schema {
        #[command(subcommand)]
        action: SchemaAction,
    },
}

#[derive(Subcommand)]
enum SchemaAction {
    /// Export the current schema as a snapshot
    Snapshot {
        /// Output format (json, yaml)
        #[arg(long, default_value = "yaml")]
        format: String,
        /// Output file path
        #[arg(long)]
        output: Option<String>,
    },
    /// Apply a schema snapshot
    Apply {
        /// Path to the snapshot file
        #[arg(long)]
        snapshot: String,
    },
}

/// Connect to the configured database
async fn connect_database() -> Result<Arc<dyn nexus_database::DatabaseBackend>, Box<dyn std::error::Error>> {
    let db_url = nexus_env::env_string_or("DB_CONNECTION_STRING", "sqlite://./data.db");
    tracing::info!(dialect = %db_url.split(':').next().unwrap_or("unknown"), "Connecting to database");
    let pool = DatabasePool::new(&db_url).await?;
    Ok(Arc::new(pool) as Arc<dyn nexus_database::DatabaseBackend>)
}

/// Initialize the application state
async fn init_app_state(
    db: Arc<dyn nexus_database::DatabaseBackend>,
) -> Result<actix_web::web::Data<AppState>, Box<dyn std::error::Error>> {
    let schema = nexus_types::schema::SchemaOverview::default();
    let emitter = Emitter::new();
    let state = AppState::new(db, schema, None, emitter);
    Ok(actix_web::web::Data::new(state))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load .env file
    nexus_env::use_env();

    // Initialize tracing
    let log_level = nexus_env::env_string_or("LOG_LEVEL", "info");
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| log_level.into()),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Start { host, port } => {
            let host = nexus_env::env_string_or("HOST", &host);
            let port_env = nexus_env::env_number_or("PORT", port as i64) as u16;

            tracing::info!("Starting Nexus server on {}:{}", host, port_env);

            // Connect to database
            let db = connect_database().await?;

            // Initialize app state
            let app_state = init_app_state(db).await?;

            // Initialize WebSocket manager
            let ws_manager = Arc::new(WebSocketManager::new());
            let ws_data = actix_web::web::Data::new(ws_manager.clone());

            // Initialize flow manager
            let flow_manager = Arc::new(nexus_flows::manager::FlowManager::new());
            nexus_flows::operations::register_all(&flow_manager).await;

            // Initialize extension manager
            let extensions_path = nexus_env::env_string_or("EXTENSIONS_PATH", "./extensions");
            let ext_manager = Arc::new(nexus_extensions::ExtensionManager::new(
                PathBuf::from(&extensions_path),
            ));
            match ext_manager.load_all().await {
                Ok(names) => {
                    if !names.is_empty() {
                        tracing::info!(count = names.len(), "Extensions loaded");
                    }
                }
                Err(e) => {
                    tracing::warn!(error = %e, "Failed to load extensions");
                }
            }

            // Configure CORS
            let cors_origin = nexus_env::env_string_or("CORS_ORIGIN", "*");

            // Build and start the HTTP server
            let server = actix_web::HttpServer::new(move || {
                // Configure CORS
                let cors = if cors_origin == "*" {
                    actix_cors::Cors::permissive()
                } else {
                    let mut cors = actix_cors::Cors::default();
                    for origin in cors_origin.split(',') {
                        cors = cors.allowed_origin(origin.trim());
                    }
                    cors.allowed_methods(vec!["GET", "POST", "PATCH", "PUT", "DELETE", "OPTIONS"])
                        .allowed_headers(vec!["Authorization", "Content-Type", "Accept"])
                        .supports_credentials()
                        .max_age(86400)
                };

                actix_web::App::new()
                    // Middleware stack (order matters: outermost first)
                    .wrap(cors)
                    .wrap(SecurityHeaders)
                    .wrap(Authentication)
                    .wrap(tracing_actix_web::TracingLogger::default())
                    // Shared state
                    .app_data(app_state.clone())
                    .app_data(ws_data.clone())
                    // API routes
                    .configure(nexus_api::configure_routes)
                    // WebSocket route
                    .configure(nexus_websocket::controllers::configure)
            })
            .bind(format!("{}:{}", host, port_env))?
            .workers(nexus_env::env_number_or("SERVER_WORKERS", 0) as usize) // 0 = auto
            .run();

            tracing::info!("Nexus is ready at http://{}:{}", host, port_env);
            tracing::info!("  API docs:  http://{}:{}/api/docs/swagger", host, port_env);
            tracing::info!("  RapiDoc:   http://{}:{}/api/docs/rapidoc", host, port_env);
            tracing::info!("  ReDoc:     http://{}:{}/api/docs/redoc", host, port_env);
            tracing::info!("  WebSocket: ws://{}:{}/websocket", host, port_env);

            server.await?;
        }
        Commands::Bootstrap {
            admin_email,
            admin_password,
        } => {
            tracing::info!("Bootstrapping Nexus...");

            let db = connect_database().await?;

            // Run migrations
            tracing::info!("Running database migrations...");
            nexus_database::migrations::run_migrations(&db).await?;

            // Seed system tables
            tracing::info!("Seeding system tables...");
            nexus_database::seeds::seed_system_tables(&db).await?;

            // Create admin user
            let email = admin_email.unwrap_or_else(|| {
                nexus_env::env_string_or("ADMIN_EMAIL", "admin@example.com")
            });
            let password = admin_password.unwrap_or_else(|| {
                nexus_env::env_string_or("ADMIN_PASSWORD", "admin")
            });

            tracing::info!(email = %email, "Creating admin user...");
            nexus_database::seeds::create_admin_user(&db, &email, &password).await?;

            tracing::info!("Bootstrap complete.");
        }
        Commands::Migrate => {
            tracing::info!("Running migrations...");
            let db = connect_database().await?;
            nexus_database::migrations::run_migrations(&db).await?;
            tracing::info!("Migrations complete.");
        }
        Commands::Schema { action } => match action {
            SchemaAction::Snapshot { format, output: _output } => {
                tracing::info!("Creating schema snapshot (format: {})...", format);
                let db = connect_database().await?;
                let snapshot = nexus_database::helpers::schema::SchemaInspector::snapshot(db.as_ref()).await?;

                let output_str = match format.as_str() {
                    "json" => serde_json::to_string_pretty(&snapshot)?,
                    _ => serde_yaml::to_string(&snapshot)?,
                };
                println!("{}", output_str);
            }
            SchemaAction::Apply { snapshot } => {
                tracing::info!("Applying schema from: {}", snapshot);
                // TODO: Diff snapshot against current schema and apply changes
            }
        },
    }

    Ok(())
}
