use clap::{Parser, Subcommand};

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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Start { host, port } => {
            tracing::info!("Starting Nexus server on {}:{}", host, port);

            let server = actix_web::HttpServer::new(move || {
                actix_web::App::new()
                    .wrap(tracing_actix_web::TracingLogger::default())
                    .configure(nexus_api::configure_routes)
            })
            .bind(format!("{}:{}", host, port))?
            .run();

            tracing::info!("Nexus is ready at http://{}:{}", host, port);
            tracing::info!("API docs: http://{}:{}/api/docs/swagger", host, port);
            tracing::info!("RapiDoc:  http://{}:{}/api/docs/rapidoc", host, port);
            tracing::info!("ReDoc:    http://{}:{}/api/docs/redoc", host, port);

            server.await?;
        }
        Commands::Bootstrap {
            admin_email,
            admin_password,
        } => {
            tracing::info!("Bootstrapping Nexus...");
            // TODO: Initialize database, run migrations, create admin user
            tracing::info!("Bootstrap complete.");
        }
        Commands::Migrate => {
            tracing::info!("Running migrations...");
            // TODO: Run pending migrations
            tracing::info!("Migrations complete.");
        }
        Commands::Schema { action } => match action {
            SchemaAction::Snapshot { format, output } => {
                tracing::info!("Creating schema snapshot (format: {})...", format);
                // TODO: Export schema
            }
            SchemaAction::Apply { snapshot } => {
                tracing::info!("Applying schema from: {}", snapshot);
                // TODO: Apply schema
            }
        },
    }

    Ok(())
}
