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
    // Try to load the current schema from the database
    let schema = match nexus_database::helpers::schema::SchemaInspector::snapshot(db.as_ref()).await {
        Ok(s) => s,
        Err(_) => nexus_types::schema::SchemaOverview::default(),
    };
    let emitter = Emitter::new();

    // Initialize storage driver
    let storage_location = nexus_env::env_string_or("STORAGE_LOCAL_ROOT", "./uploads");
    let storage: Arc<dyn nexus_storage::StorageDriver> =
        Arc::new(nexus_storage::drivers::local::LocalDriver::new(storage_location));

    let state = AppState::new(db, schema, None, emitter)
        .with_storage(storage);
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

            // Initialize WebSocket manager with db/schema/emitter for WS CRUD
            let ws_manager = Arc::new(
                WebSocketManager::new()
                    .with_db(app_state.db.clone())
                    .with_schema(app_state.schema.clone())
                    .with_emitter(app_state.emitter.clone()),
            );
            let ws_data = actix_web::web::Data::new(ws_manager.clone());

            // Initialize flow manager and load flows from database
            let flow_manager = Arc::new(nexus_flows::manager::FlowManager::new());
            nexus_flows::operations::register_all(&flow_manager).await;

            // Load flows from the database
            {
                let ctx = app_state.service_context(None).await;
                let flows_svc = nexus_services::items::ItemsService::new("directus_flows", ctx.clone());
                let ops_svc = nexus_services::items::ItemsService::new("directus_operations", ctx);

                let flow_defs: Vec<nexus_flows::FlowDefinition> = flows_svc
                    .read_by_query(nexus_types::query::Query::default(), None)
                    .await
                    .unwrap_or_default()
                    .into_iter()
                    .filter_map(|v| serde_json::from_value(v).ok())
                    .collect();

                let op_defs: Vec<nexus_flows::OperationDefinition> = ops_svc
                    .read_by_query(nexus_types::query::Query::default(), None)
                    .await
                    .unwrap_or_default()
                    .into_iter()
                    .filter_map(|v| serde_json::from_value(v).ok())
                    .collect();

                flow_manager.load_flows(flow_defs, op_defs).await;
            }

            // Start the cron scheduler for scheduled flows
            {
                let fm = flow_manager.clone();
                tokio::spawn(async move {
                    if let Err(e) = nexus_flows::manager::FlowManager::start_cron_scheduler(fm).await {
                        tracing::warn!(error = %e, "Failed to start cron scheduler");
                    }
                });
            }

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

                let db = connect_database().await?;

                // Read snapshot file
                let snapshot_content = std::fs::read_to_string(&snapshot)
                    .map_err(|e| format!("Failed to read snapshot file '{}': {}", snapshot, e))?;

                // Parse snapshot (detect format from extension)
                let target_schema: nexus_types::schema::SchemaOverview =
                    if snapshot.ends_with(".json") {
                        serde_json::from_str(&snapshot_content)
                            .map_err(|e| format!("Failed to parse JSON snapshot: {}", e))?
                    } else {
                        serde_yaml::from_str(&snapshot_content)
                            .map_err(|e| format!("Failed to parse YAML snapshot: {}", e))?
                    };

                // Get current schema
                let current_schema =
                    nexus_database::helpers::schema::SchemaInspector::snapshot(db.as_ref()).await?;

                let mut ddl_statements: Vec<String> = Vec::new();

                // 1. Create new collections (tables)
                for (name, target_col) in &target_schema.collections {
                    if !current_schema.collections.contains_key(name) {
                        tracing::info!(collection = %name, "Creating collection");
                        let mut col_defs: Vec<String> = Vec::new();
                        for (field_name, field) in &target_col.fields {
                            let db_type = field
                                .db_type
                                .clone()
                                .unwrap_or_else(|| field_type_to_sql(&field.field_type));
                            let mut col_def = format!(
                                "{} {}",
                                db.quote_identifier(field_name),
                                db_type
                            );
                            if !field.nullable {
                                col_def.push_str(" NOT NULL");
                            }
                            if let Some(ref def) = field.default_value {
                                if let Some(s) = def.as_str() {
                                    col_def.push_str(&format!(" DEFAULT '{}'", s));
                                } else if !def.is_null() {
                                    col_def.push_str(&format!(" DEFAULT {}", def));
                                }
                            }
                            if field_name == &target_col.primary {
                                col_def.push_str(" PRIMARY KEY");
                            }
                            col_defs.push(col_def);
                        }
                        ddl_statements.push(format!(
                            "CREATE TABLE {} ({})",
                            db.quote_identifier(name),
                            col_defs.join(", ")
                        ));
                    }
                }

                // 2. Add new fields to existing collections
                for (name, target_col) in &target_schema.collections {
                    if let Some(current_col) = current_schema.collections.get(name) {
                        for (field_name, field) in &target_col.fields {
                            if !current_col.fields.contains_key(field_name) {
                                tracing::info!(collection = %name, field = %field_name, "Adding field");
                                let db_type = field
                                    .db_type
                                    .clone()
                                    .unwrap_or_else(|| field_type_to_sql(&field.field_type));
                                let nullable = if field.nullable { "" } else { " NOT NULL" };
                                ddl_statements.push(format!(
                                    "ALTER TABLE {} ADD COLUMN {} {}{}",
                                    db.quote_identifier(name),
                                    db.quote_identifier(field_name),
                                    db_type,
                                    nullable,
                                ));
                            }
                        }
                    }
                }

                // 3. Drop user collections that no longer exist in the target
                for name in current_schema.collections.keys() {
                    if !target_schema.collections.contains_key(name)
                        && !name.starts_with("directus_")
                    {
                        tracing::info!(collection = %name, "Dropping collection");
                        ddl_statements.push(format!(
                            "DROP TABLE IF EXISTS {}",
                            db.quote_identifier(name)
                        ));
                    }
                }

                // 4. Drop fields that no longer exist in the target
                for (name, current_col) in &current_schema.collections {
                    if let Some(target_col) = target_schema.collections.get(name) {
                        for field_name in current_col.fields.keys() {
                            if !target_col.fields.contains_key(field_name) {
                                tracing::info!(collection = %name, field = %field_name, "Dropping field");
                                ddl_statements.push(format!(
                                    "ALTER TABLE {} DROP COLUMN {}",
                                    db.quote_identifier(name),
                                    db.quote_identifier(field_name),
                                ));
                            }
                        }
                    }
                }

                // Execute all DDL statements
                if ddl_statements.is_empty() {
                    tracing::info!("Schema is already up to date. No changes needed.");
                } else {
                    tracing::info!(
                        changes = ddl_statements.len(),
                        "Applying schema changes..."
                    );
                    for stmt in &ddl_statements {
                        tracing::debug!(sql = %stmt, "Executing DDL");
                        db.execute(stmt, &[]).await.map_err(|e| {
                            format!("Failed to execute DDL '{}': {}", stmt, e)
                        })?;
                    }
                    tracing::info!("Schema apply complete.");
                }
            }
        },
    }

    Ok(())
}

/// Map a FieldType to a default SQL column type string
fn field_type_to_sql(ft: &nexus_types::fields::FieldType) -> String {
    use nexus_types::fields::FieldType;
    match ft {
        FieldType::Integer => "integer".to_string(),
        FieldType::BigInteger => "bigint".to_string(),
        FieldType::Float | FieldType::Decimal => "real".to_string(),
        FieldType::Boolean => "boolean".to_string(),
        FieldType::String | FieldType::Hash | FieldType::Csv => "varchar(255)".to_string(),
        FieldType::Text => "text".to_string(),
        FieldType::Date => "date".to_string(),
        FieldType::Time => "time".to_string(),
        FieldType::DateTime | FieldType::Timestamp => "timestamp".to_string(),
        FieldType::Json => "jsonb".to_string(),
        FieldType::Uuid => "uuid".to_string(),
        FieldType::Binary => "bytea".to_string(),
        FieldType::Geometry
        | FieldType::GeometryPoint
        | FieldType::GeometryLineString
        | FieldType::GeometryPolygon
        | FieldType::GeometryMultiPoint
        | FieldType::GeometryMultiLineString
        | FieldType::GeometryMultiPolygon => "geometry".to_string(),
        FieldType::Alias | FieldType::Unknown => "text".to_string(),
    }
}
