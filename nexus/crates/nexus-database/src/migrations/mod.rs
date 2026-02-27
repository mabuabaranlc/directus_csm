use crate::{DatabaseBackend, DatabaseError, Dialect};
use std::sync::Arc;
use tracing;

/// Run all pending database migrations
pub async fn run_migrations(db: &Arc<dyn DatabaseBackend>) -> Result<(), DatabaseError> {
    // Ensure the migrations tracking table exists
    create_migrations_table(db.as_ref()).await?;

    // Get list of already applied migrations
    let applied = get_applied_migrations(db.as_ref()).await?;

    // Run each pending migration in order
    let all_migrations = get_all_migrations();
    let mut count = 0;

    for migration in &all_migrations {
        if !applied.contains(&migration.version) {
            tracing::info!(
                version = %migration.version,
                name = %migration.name,
                "Running migration"
            );
            run_migration(db.as_ref(), migration).await?;
            record_migration(db.as_ref(), migration).await?;
            count += 1;
        }
    }

    if count > 0 {
        tracing::info!(count, "Migrations applied successfully");
    } else {
        tracing::info!("No pending migrations");
    }

    Ok(())
}

struct Migration {
    version: String,
    name: String,
    sql_postgres: &'static str,
    sql_mysql: &'static str,
    sql_sqlite: &'static str,
}

async fn create_migrations_table(db: &dyn DatabaseBackend) -> Result<(), DatabaseError> {
    let sql = match db.dialect() {
        Dialect::Postgres | Dialect::CockroachDB => {
            "CREATE TABLE IF NOT EXISTS nexus_migrations (
                version VARCHAR(255) PRIMARY KEY,
                name VARCHAR(255) NOT NULL,
                timestamp TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )"
        }
        Dialect::MySQL => {
            "CREATE TABLE IF NOT EXISTS nexus_migrations (
                version VARCHAR(255) PRIMARY KEY,
                name VARCHAR(255) NOT NULL,
                timestamp TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )"
        }
        Dialect::SQLite => {
            "CREATE TABLE IF NOT EXISTS nexus_migrations (
                version TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                timestamp TEXT DEFAULT (datetime('now'))
            )"
        }
        Dialect::MSSQL => {
            "IF NOT EXISTS (SELECT * FROM sys.tables WHERE name = 'nexus_migrations')
            CREATE TABLE nexus_migrations (
                version NVARCHAR(255) PRIMARY KEY,
                name NVARCHAR(255) NOT NULL,
                timestamp DATETIME2 DEFAULT GETUTCDATE()
            )"
        }
        Dialect::Oracle => {
            "BEGIN
                EXECUTE IMMEDIATE 'CREATE TABLE nexus_migrations (
                    version VARCHAR2(255) PRIMARY KEY,
                    name VARCHAR2(255) NOT NULL,
                    timestamp TIMESTAMP DEFAULT SYSTIMESTAMP
                )';
            EXCEPTION WHEN OTHERS THEN NULL; END;"
        }
    };

    db.execute(sql, &[]).await?;
    Ok(())
}

async fn get_applied_migrations(db: &dyn DatabaseBackend) -> Result<Vec<String>, DatabaseError> {
    let rows = db
        .query("SELECT version FROM nexus_migrations ORDER BY version", &[])
        .await?;

    Ok(rows
        .into_iter()
        .filter_map(|row| {
            row.get("version")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        })
        .collect())
}

async fn run_migration(db: &dyn DatabaseBackend, migration: &Migration) -> Result<(), DatabaseError> {
    let sql = match db.dialect() {
        Dialect::Postgres | Dialect::CockroachDB => migration.sql_postgres,
        Dialect::MySQL => migration.sql_mysql,
        Dialect::SQLite => migration.sql_sqlite,
        Dialect::MSSQL | Dialect::Oracle => migration.sql_postgres, // Fallback
    };

    // Split by semicolons and execute each statement (skip empty)
    for statement in sql.split(';') {
        let stmt = statement.trim();
        if !stmt.is_empty() {
            db.execute(stmt, &[])
                .await
                .map_err(|e| DatabaseError::Migration(format!(
                    "Migration {} failed: {}",
                    migration.version, e
                )))?;
        }
    }

    Ok(())
}

async fn record_migration(db: &dyn DatabaseBackend, migration: &Migration) -> Result<(), DatabaseError> {
    db.execute(
        "INSERT INTO nexus_migrations (version, name) VALUES ($1, $2)",
        &[
            crate::SqlValue::Text(migration.version.clone()),
            crate::SqlValue::Text(migration.name.clone()),
        ],
    )
    .await?;
    Ok(())
}

/// Get all migrations in order
fn get_all_migrations() -> Vec<Migration> {
    vec![
        Migration {
            version: "20240101A".to_string(),
            name: "Create Roles Table".to_string(),
            sql_postgres: "
                CREATE TABLE IF NOT EXISTS nexus_roles (
                    id UUID PRIMARY KEY,
                    name VARCHAR(100) NOT NULL,
                    icon VARCHAR(64) DEFAULT 'supervised_user_circle',
                    description TEXT,
                    parent UUID REFERENCES nexus_roles(id),
                    enforce_tfa BOOLEAN DEFAULT FALSE,
                    admin_access BOOLEAN DEFAULT FALSE,
                    app_access BOOLEAN DEFAULT TRUE,
                    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
                )",
            sql_mysql: "
                CREATE TABLE IF NOT EXISTS nexus_roles (
                    id CHAR(36) PRIMARY KEY,
                    name VARCHAR(100) NOT NULL,
                    icon VARCHAR(64) DEFAULT 'supervised_user_circle',
                    description TEXT,
                    parent CHAR(36),
                    enforce_tfa BOOLEAN DEFAULT FALSE,
                    admin_access BOOLEAN DEFAULT FALSE,
                    app_access BOOLEAN DEFAULT TRUE,
                    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP
                )",
            sql_sqlite: "
                CREATE TABLE IF NOT EXISTS nexus_roles (
                    id TEXT PRIMARY KEY,
                    name TEXT NOT NULL,
                    icon TEXT DEFAULT 'supervised_user_circle',
                    description TEXT,
                    parent TEXT REFERENCES nexus_roles(id),
                    enforce_tfa INTEGER DEFAULT 0,
                    admin_access INTEGER DEFAULT 0,
                    app_access INTEGER DEFAULT 1,
                    created_at TEXT DEFAULT (datetime('now')),
                    updated_at TEXT DEFAULT (datetime('now'))
                )",
        },
        Migration {
            version: "20240101B".to_string(),
            name: "Create Users Table".to_string(),
            sql_postgres: "
                CREATE TABLE IF NOT EXISTS nexus_users (
                    id UUID PRIMARY KEY,
                    first_name VARCHAR(50),
                    last_name VARCHAR(50),
                    email VARCHAR(128) UNIQUE,
                    password VARCHAR(255),
                    location VARCHAR(255),
                    title VARCHAR(50),
                    description TEXT,
                    tags JSON,
                    avatar UUID,
                    language VARCHAR(16) DEFAULT 'en-US',
                    tfa_secret VARCHAR(255),
                    status VARCHAR(16) DEFAULT 'active',
                    role UUID REFERENCES nexus_roles(id),
                    token VARCHAR(255) UNIQUE,
                    last_access TIMESTAMP,
                    last_page VARCHAR(255),
                    provider VARCHAR(128) DEFAULT 'default',
                    external_identifier VARCHAR(255),
                    auth_data JSON,
                    email_notifications BOOLEAN DEFAULT TRUE,
                    appearance VARCHAR(32) DEFAULT 'auto',
                    theme_dark VARCHAR(255),
                    theme_light VARCHAR(255),
                    theme_dark_overrides JSON,
                    theme_light_overrides JSON,
                    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
                )",
            sql_mysql: "
                CREATE TABLE IF NOT EXISTS nexus_users (
                    id CHAR(36) PRIMARY KEY,
                    first_name VARCHAR(50),
                    last_name VARCHAR(50),
                    email VARCHAR(128) UNIQUE,
                    password VARCHAR(255),
                    location VARCHAR(255),
                    title VARCHAR(50),
                    description TEXT,
                    tags JSON,
                    avatar CHAR(36),
                    language VARCHAR(16) DEFAULT 'en-US',
                    tfa_secret VARCHAR(255),
                    status VARCHAR(16) DEFAULT 'active',
                    role CHAR(36),
                    token VARCHAR(255) UNIQUE,
                    last_access TIMESTAMP NULL,
                    last_page VARCHAR(255),
                    provider VARCHAR(128) DEFAULT 'default',
                    external_identifier VARCHAR(255),
                    auth_data JSON,
                    email_notifications BOOLEAN DEFAULT TRUE,
                    appearance VARCHAR(32) DEFAULT 'auto',
                    theme_dark VARCHAR(255),
                    theme_light VARCHAR(255),
                    theme_dark_overrides JSON,
                    theme_light_overrides JSON,
                    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP
                )",
            sql_sqlite: "
                CREATE TABLE IF NOT EXISTS nexus_users (
                    id TEXT PRIMARY KEY,
                    first_name TEXT,
                    last_name TEXT,
                    email TEXT UNIQUE,
                    password TEXT,
                    location TEXT,
                    title TEXT,
                    description TEXT,
                    tags TEXT,
                    avatar TEXT,
                    language TEXT DEFAULT 'en-US',
                    tfa_secret TEXT,
                    status TEXT DEFAULT 'active',
                    role TEXT REFERENCES nexus_roles(id),
                    token TEXT UNIQUE,
                    last_access TEXT,
                    last_page TEXT,
                    provider TEXT DEFAULT 'default',
                    external_identifier TEXT,
                    auth_data TEXT,
                    email_notifications INTEGER DEFAULT 1,
                    appearance TEXT DEFAULT 'auto',
                    theme_dark TEXT,
                    theme_light TEXT,
                    theme_dark_overrides TEXT,
                    theme_light_overrides TEXT,
                    created_at TEXT DEFAULT (datetime('now')),
                    updated_at TEXT DEFAULT (datetime('now'))
                )",
        },
        Migration {
            version: "20240101C".to_string(),
            name: "Create Sessions Table".to_string(),
            sql_postgres: "
                CREATE TABLE IF NOT EXISTS nexus_sessions (
                    token VARCHAR(64) PRIMARY KEY,
                    \"user\" UUID REFERENCES nexus_users(id) ON DELETE CASCADE,
                    expires TIMESTAMP NOT NULL,
                    ip VARCHAR(255),
                    user_agent TEXT,
                    share UUID,
                    origin VARCHAR(255),
                    next_token VARCHAR(64),
                    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
                )",
            sql_mysql: "
                CREATE TABLE IF NOT EXISTS nexus_sessions (
                    token VARCHAR(64) PRIMARY KEY,
                    user CHAR(36),
                    expires TIMESTAMP NOT NULL,
                    ip VARCHAR(255),
                    user_agent TEXT,
                    share CHAR(36),
                    origin VARCHAR(255),
                    next_token VARCHAR(64),
                    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
                )",
            sql_sqlite: "
                CREATE TABLE IF NOT EXISTS nexus_sessions (
                    token TEXT PRIMARY KEY,
                    user TEXT REFERENCES nexus_users(id) ON DELETE CASCADE,
                    expires TEXT NOT NULL,
                    ip TEXT,
                    user_agent TEXT,
                    share TEXT,
                    origin TEXT,
                    next_token TEXT,
                    created_at TEXT DEFAULT (datetime('now'))
                )",
        },
        Migration {
            version: "20240101D".to_string(),
            name: "Create Collections Table".to_string(),
            sql_postgres: "
                CREATE TABLE IF NOT EXISTS nexus_collections (
                    collection VARCHAR(64) PRIMARY KEY,
                    icon VARCHAR(64),
                    note TEXT,
                    display_template VARCHAR(255),
                    hidden BOOLEAN DEFAULT FALSE,
                    singleton BOOLEAN DEFAULT FALSE,
                    translations JSON,
                    archive_field VARCHAR(64),
                    archive_app_filter BOOLEAN DEFAULT TRUE,
                    archive_value VARCHAR(255),
                    unarchive_value VARCHAR(255),
                    sort_field VARCHAR(64),
                    accountability VARCHAR(16) DEFAULT 'all',
                    color VARCHAR(16),
                    item_duplication_fields JSON,
                    sort INT,
                    \"group\" VARCHAR(64) REFERENCES nexus_collections(collection),
                    collapse VARCHAR(16) DEFAULT 'open',
                    preview_url VARCHAR(255),
                    versioning BOOLEAN DEFAULT FALSE
                )",
            sql_mysql: "
                CREATE TABLE IF NOT EXISTS nexus_collections (
                    collection VARCHAR(64) PRIMARY KEY,
                    icon VARCHAR(64),
                    note TEXT,
                    display_template VARCHAR(255),
                    hidden BOOLEAN DEFAULT FALSE,
                    singleton BOOLEAN DEFAULT FALSE,
                    translations JSON,
                    archive_field VARCHAR(64),
                    archive_app_filter BOOLEAN DEFAULT TRUE,
                    archive_value VARCHAR(255),
                    unarchive_value VARCHAR(255),
                    sort_field VARCHAR(64),
                    accountability VARCHAR(16) DEFAULT 'all',
                    color VARCHAR(16),
                    item_duplication_fields JSON,
                    sort INT,
                    `group` VARCHAR(64),
                    collapse VARCHAR(16) DEFAULT 'open',
                    preview_url VARCHAR(255),
                    versioning BOOLEAN DEFAULT FALSE
                )",
            sql_sqlite: "
                CREATE TABLE IF NOT EXISTS nexus_collections (
                    collection TEXT PRIMARY KEY,
                    icon TEXT,
                    note TEXT,
                    display_template TEXT,
                    hidden INTEGER DEFAULT 0,
                    singleton INTEGER DEFAULT 0,
                    translations TEXT,
                    archive_field TEXT,
                    archive_app_filter INTEGER DEFAULT 1,
                    archive_value TEXT,
                    unarchive_value TEXT,
                    sort_field TEXT,
                    accountability TEXT DEFAULT 'all',
                    color TEXT,
                    item_duplication_fields TEXT,
                    sort INTEGER,
                    \"group\" TEXT REFERENCES nexus_collections(collection),
                    collapse TEXT DEFAULT 'open',
                    preview_url TEXT,
                    versioning INTEGER DEFAULT 0
                )",
        },
        Migration {
            version: "20240101E".to_string(),
            name: "Create Fields Table".to_string(),
            sql_postgres: "
                CREATE TABLE IF NOT EXISTS nexus_fields (
                    id SERIAL PRIMARY KEY,
                    collection VARCHAR(64) NOT NULL REFERENCES nexus_collections(collection) ON DELETE CASCADE,
                    field VARCHAR(64) NOT NULL,
                    special VARCHAR(64),
                    interface VARCHAR(64),
                    options JSON,
                    display VARCHAR(64),
                    display_options JSON,
                    readonly BOOLEAN DEFAULT FALSE,
                    hidden BOOLEAN DEFAULT FALSE,
                    sort INT,
                    width VARCHAR(16) DEFAULT 'full',
                    translations JSON,
                    note TEXT,
                    conditions JSON,
                    required BOOLEAN DEFAULT FALSE,
                    \"group\" VARCHAR(64),
                    validation JSON,
                    validation_message TEXT,
                    UNIQUE(collection, field)
                )",
            sql_mysql: "
                CREATE TABLE IF NOT EXISTS nexus_fields (
                    id INT AUTO_INCREMENT PRIMARY KEY,
                    collection VARCHAR(64) NOT NULL,
                    field VARCHAR(64) NOT NULL,
                    special VARCHAR(64),
                    interface VARCHAR(64),
                    options JSON,
                    display VARCHAR(64),
                    display_options JSON,
                    readonly BOOLEAN DEFAULT FALSE,
                    hidden BOOLEAN DEFAULT FALSE,
                    sort INT,
                    width VARCHAR(16) DEFAULT 'full',
                    translations JSON,
                    note TEXT,
                    conditions JSON,
                    required BOOLEAN DEFAULT FALSE,
                    `group` VARCHAR(64),
                    validation JSON,
                    validation_message TEXT,
                    UNIQUE KEY unique_collection_field (collection, field)
                )",
            sql_sqlite: "
                CREATE TABLE IF NOT EXISTS nexus_fields (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    collection TEXT NOT NULL REFERENCES nexus_collections(collection) ON DELETE CASCADE,
                    field TEXT NOT NULL,
                    special TEXT,
                    interface TEXT,
                    options TEXT,
                    display TEXT,
                    display_options TEXT,
                    readonly INTEGER DEFAULT 0,
                    hidden INTEGER DEFAULT 0,
                    sort INTEGER,
                    width TEXT DEFAULT 'full',
                    translations TEXT,
                    note TEXT,
                    conditions TEXT,
                    required INTEGER DEFAULT 0,
                    \"group\" TEXT,
                    validation TEXT,
                    validation_message TEXT,
                    UNIQUE(collection, field)
                )",
        },
        Migration {
            version: "20240101F".to_string(),
            name: "Create Relations Table".to_string(),
            sql_postgres: "
                CREATE TABLE IF NOT EXISTS nexus_relations (
                    id SERIAL PRIMARY KEY,
                    many_collection VARCHAR(64) NOT NULL,
                    many_field VARCHAR(64) NOT NULL,
                    one_collection VARCHAR(64),
                    one_field VARCHAR(64),
                    one_collection_field VARCHAR(64),
                    one_allowed_collections TEXT,
                    junction_field VARCHAR(64),
                    sort_field VARCHAR(64),
                    one_deselect_action VARCHAR(16) DEFAULT 'nullify'
                )",
            sql_mysql: "
                CREATE TABLE IF NOT EXISTS nexus_relations (
                    id INT AUTO_INCREMENT PRIMARY KEY,
                    many_collection VARCHAR(64) NOT NULL,
                    many_field VARCHAR(64) NOT NULL,
                    one_collection VARCHAR(64),
                    one_field VARCHAR(64),
                    one_collection_field VARCHAR(64),
                    one_allowed_collections TEXT,
                    junction_field VARCHAR(64),
                    sort_field VARCHAR(64),
                    one_deselect_action VARCHAR(16) DEFAULT 'nullify'
                )",
            sql_sqlite: "
                CREATE TABLE IF NOT EXISTS nexus_relations (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    many_collection TEXT NOT NULL,
                    many_field TEXT NOT NULL,
                    one_collection TEXT,
                    one_field TEXT,
                    one_collection_field TEXT,
                    one_allowed_collections TEXT,
                    junction_field TEXT,
                    sort_field TEXT,
                    one_deselect_action TEXT DEFAULT 'nullify'
                )",
        },
        Migration {
            version: "20240101G".to_string(),
            name: "Create Policies Table".to_string(),
            sql_postgres: "
                CREATE TABLE IF NOT EXISTS nexus_policies (
                    id UUID PRIMARY KEY,
                    name VARCHAR(100) NOT NULL,
                    icon VARCHAR(64) DEFAULT 'badge',
                    description TEXT,
                    ip_access TEXT,
                    enforce_tfa BOOLEAN DEFAULT FALSE,
                    admin_access BOOLEAN DEFAULT FALSE,
                    app_access BOOLEAN DEFAULT FALSE,
                    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
                )",
            sql_mysql: "
                CREATE TABLE IF NOT EXISTS nexus_policies (
                    id CHAR(36) PRIMARY KEY,
                    name VARCHAR(100) NOT NULL,
                    icon VARCHAR(64) DEFAULT 'badge',
                    description TEXT,
                    ip_access TEXT,
                    enforce_tfa BOOLEAN DEFAULT FALSE,
                    admin_access BOOLEAN DEFAULT FALSE,
                    app_access BOOLEAN DEFAULT FALSE,
                    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP
                )",
            sql_sqlite: "
                CREATE TABLE IF NOT EXISTS nexus_policies (
                    id TEXT PRIMARY KEY,
                    name TEXT NOT NULL,
                    icon TEXT DEFAULT 'badge',
                    description TEXT,
                    ip_access TEXT,
                    enforce_tfa INTEGER DEFAULT 0,
                    admin_access INTEGER DEFAULT 0,
                    app_access INTEGER DEFAULT 0,
                    created_at TEXT DEFAULT (datetime('now')),
                    updated_at TEXT DEFAULT (datetime('now'))
                )",
        },
        Migration {
            version: "20240101H".to_string(),
            name: "Create Permissions Table".to_string(),
            sql_postgres: "
                CREATE TABLE IF NOT EXISTS nexus_permissions (
                    id SERIAL PRIMARY KEY,
                    policy UUID REFERENCES nexus_policies(id) ON DELETE CASCADE,
                    collection VARCHAR(64) NOT NULL,
                    action VARCHAR(16) NOT NULL,
                    permissions JSON,
                    validation JSON,
                    presets JSON,
                    fields TEXT
                )",
            sql_mysql: "
                CREATE TABLE IF NOT EXISTS nexus_permissions (
                    id INT AUTO_INCREMENT PRIMARY KEY,
                    policy CHAR(36),
                    collection VARCHAR(64) NOT NULL,
                    action VARCHAR(16) NOT NULL,
                    permissions JSON,
                    validation JSON,
                    presets JSON,
                    fields TEXT
                )",
            sql_sqlite: "
                CREATE TABLE IF NOT EXISTS nexus_permissions (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    policy TEXT,
                    collection TEXT NOT NULL,
                    action TEXT NOT NULL,
                    permissions TEXT,
                    validation TEXT,
                    presets TEXT,
                    fields TEXT
                )",
        },
        Migration {
            version: "20240101I".to_string(),
            name: "Create Access Table".to_string(),
            sql_postgres: "
                CREATE TABLE IF NOT EXISTS nexus_access (
                    id UUID PRIMARY KEY,
                    role UUID REFERENCES nexus_roles(id) ON DELETE CASCADE,
                    \"user\" UUID REFERENCES nexus_users(id) ON DELETE CASCADE,
                    policy UUID NOT NULL REFERENCES nexus_policies(id) ON DELETE CASCADE,
                    sort INT
                )",
            sql_mysql: "
                CREATE TABLE IF NOT EXISTS nexus_access (
                    id CHAR(36) PRIMARY KEY,
                    role CHAR(36),
                    user CHAR(36),
                    policy CHAR(36) NOT NULL,
                    sort INT
                )",
            sql_sqlite: "
                CREATE TABLE IF NOT EXISTS nexus_access (
                    id TEXT PRIMARY KEY,
                    role TEXT REFERENCES nexus_roles(id) ON DELETE CASCADE,
                    user TEXT REFERENCES nexus_users(id) ON DELETE CASCADE,
                    policy TEXT NOT NULL REFERENCES nexus_policies(id) ON DELETE CASCADE,
                    sort INTEGER
                )",
        },
        Migration {
            version: "20240101J".to_string(),
            name: "Create Files and Folders Tables".to_string(),
            sql_postgres: "
                CREATE TABLE IF NOT EXISTS nexus_folders (
                    id UUID PRIMARY KEY,
                    name VARCHAR(255) NOT NULL,
                    parent UUID REFERENCES nexus_folders(id)
                );
                CREATE TABLE IF NOT EXISTS nexus_files (
                    id UUID PRIMARY KEY,
                    storage VARCHAR(255) NOT NULL DEFAULT 'local',
                    filename_disk VARCHAR(255),
                    filename_download VARCHAR(255) NOT NULL,
                    title VARCHAR(255),
                    type VARCHAR(255),
                    folder UUID REFERENCES nexus_folders(id),
                    uploaded_by UUID REFERENCES nexus_users(id),
                    created_on TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                    modified_by UUID REFERENCES nexus_users(id),
                    modified_on TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                    charset VARCHAR(50),
                    filesize BIGINT DEFAULT 0,
                    width INT,
                    height INT,
                    duration INT,
                    embed VARCHAR(200),
                    description TEXT,
                    location VARCHAR(200),
                    tags JSON,
                    metadata JSON,
                    focal_point_x INT,
                    focal_point_y INT,
                    tus_id VARCHAR(64),
                    tus_data JSON,
                    uploaded_on TIMESTAMP DEFAULT CURRENT_TIMESTAMP
                )",
            sql_mysql: "
                CREATE TABLE IF NOT EXISTS nexus_folders (
                    id CHAR(36) PRIMARY KEY,
                    name VARCHAR(255) NOT NULL,
                    parent CHAR(36)
                );
                CREATE TABLE IF NOT EXISTS nexus_files (
                    id CHAR(36) PRIMARY KEY,
                    storage VARCHAR(255) NOT NULL DEFAULT 'local',
                    filename_disk VARCHAR(255),
                    filename_download VARCHAR(255) NOT NULL,
                    title VARCHAR(255),
                    type VARCHAR(255),
                    folder CHAR(36),
                    uploaded_by CHAR(36),
                    created_on TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                    modified_by CHAR(36),
                    modified_on TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
                    charset VARCHAR(50),
                    filesize BIGINT DEFAULT 0,
                    width INT,
                    height INT,
                    duration INT,
                    embed VARCHAR(200),
                    description TEXT,
                    location VARCHAR(200),
                    tags JSON,
                    metadata JSON,
                    focal_point_x INT,
                    focal_point_y INT,
                    tus_id VARCHAR(64),
                    tus_data JSON,
                    uploaded_on TIMESTAMP DEFAULT CURRENT_TIMESTAMP
                )",
            sql_sqlite: "
                CREATE TABLE IF NOT EXISTS nexus_folders (
                    id TEXT PRIMARY KEY,
                    name TEXT NOT NULL,
                    parent TEXT REFERENCES nexus_folders(id)
                );
                CREATE TABLE IF NOT EXISTS nexus_files (
                    id TEXT PRIMARY KEY,
                    storage TEXT NOT NULL DEFAULT 'local',
                    filename_disk TEXT,
                    filename_download TEXT NOT NULL,
                    title TEXT,
                    type TEXT,
                    folder TEXT REFERENCES nexus_folders(id),
                    uploaded_by TEXT REFERENCES nexus_users(id),
                    created_on TEXT DEFAULT (datetime('now')),
                    modified_by TEXT REFERENCES nexus_users(id),
                    modified_on TEXT DEFAULT (datetime('now')),
                    charset TEXT,
                    filesize INTEGER DEFAULT 0,
                    width INTEGER,
                    height INTEGER,
                    duration INTEGER,
                    embed TEXT,
                    description TEXT,
                    location TEXT,
                    tags TEXT,
                    metadata TEXT,
                    focal_point_x INTEGER,
                    focal_point_y INTEGER,
                    tus_id TEXT,
                    tus_data TEXT,
                    uploaded_on TEXT DEFAULT (datetime('now'))
                )",
        },
        Migration {
            version: "20240101K".to_string(),
            name: "Create Activity and Revisions Tables".to_string(),
            sql_postgres: "
                CREATE TABLE IF NOT EXISTS nexus_activity (
                    id SERIAL PRIMARY KEY,
                    action VARCHAR(45) NOT NULL,
                    \"user\" UUID,
                    timestamp TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                    ip VARCHAR(50),
                    user_agent TEXT,
                    collection VARCHAR(64) NOT NULL,
                    item VARCHAR(255) NOT NULL,
                    comment TEXT,
                    origin VARCHAR(255)
                );
                CREATE TABLE IF NOT EXISTS nexus_revisions (
                    id SERIAL PRIMARY KEY,
                    activity INT REFERENCES nexus_activity(id) ON DELETE CASCADE,
                    collection VARCHAR(64) NOT NULL,
                    item VARCHAR(255) NOT NULL,
                    data JSON,
                    delta JSON,
                    parent INT REFERENCES nexus_revisions(id),
                    version UUID
                )",
            sql_mysql: "
                CREATE TABLE IF NOT EXISTS nexus_activity (
                    id INT AUTO_INCREMENT PRIMARY KEY,
                    action VARCHAR(45) NOT NULL,
                    user CHAR(36),
                    timestamp TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                    ip VARCHAR(50),
                    user_agent TEXT,
                    collection VARCHAR(64) NOT NULL,
                    item VARCHAR(255) NOT NULL,
                    comment TEXT,
                    origin VARCHAR(255)
                );
                CREATE TABLE IF NOT EXISTS nexus_revisions (
                    id INT AUTO_INCREMENT PRIMARY KEY,
                    activity INT,
                    collection VARCHAR(64) NOT NULL,
                    item VARCHAR(255) NOT NULL,
                    data JSON,
                    delta JSON,
                    parent INT,
                    version CHAR(36)
                )",
            sql_sqlite: "
                CREATE TABLE IF NOT EXISTS nexus_activity (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    action TEXT NOT NULL,
                    user TEXT,
                    timestamp TEXT DEFAULT (datetime('now')),
                    ip TEXT,
                    user_agent TEXT,
                    collection TEXT NOT NULL,
                    item TEXT NOT NULL,
                    comment TEXT,
                    origin TEXT
                );
                CREATE TABLE IF NOT EXISTS nexus_revisions (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    activity INTEGER REFERENCES nexus_activity(id) ON DELETE CASCADE,
                    collection TEXT NOT NULL,
                    item TEXT NOT NULL,
                    data TEXT,
                    delta TEXT,
                    parent INTEGER REFERENCES nexus_revisions(id),
                    version TEXT
                )",
        },
        Migration {
            version: "20240101L".to_string(),
            name: "Create Settings and Presets Tables".to_string(),
            sql_postgres: "
                CREATE TABLE IF NOT EXISTS nexus_settings (
                    id SERIAL PRIMARY KEY,
                    project_name VARCHAR(100) DEFAULT 'Nexus',
                    project_url VARCHAR(255),
                    project_color VARCHAR(16) DEFAULT '#6644FF',
                    project_logo UUID,
                    public_foreground UUID,
                    public_background UUID,
                    public_note TEXT,
                    auth_login_attempts INT DEFAULT 25,
                    auth_password_policy VARCHAR(100),
                    storage_asset_transform VARCHAR(16) DEFAULT 'all',
                    storage_asset_presets JSON,
                    custom_css TEXT,
                    storage_default_folder UUID,
                    basemaps JSON,
                    mapbox_key VARCHAR(255),
                    module_bar JSON,
                    project_descriptor VARCHAR(100),
                    default_language VARCHAR(16) DEFAULT 'en-US',
                    custom_aspect_ratios JSON,
                    public_favicon UUID,
                    default_appearance VARCHAR(32) DEFAULT 'auto',
                    default_theme_light VARCHAR(255),
                    default_theme_dark VARCHAR(255),
                    theme_light_overrides JSON,
                    theme_dark_overrides JSON,
                    report_error_url VARCHAR(255),
                    report_bug_url VARCHAR(255),
                    report_feature_url VARCHAR(255)
                );
                CREATE TABLE IF NOT EXISTS nexus_presets (
                    id SERIAL PRIMARY KEY,
                    bookmark VARCHAR(255),
                    \"user\" UUID REFERENCES nexus_users(id) ON DELETE CASCADE,
                    role UUID REFERENCES nexus_roles(id) ON DELETE CASCADE,
                    collection VARCHAR(64),
                    search VARCHAR(100),
                    layout VARCHAR(100) DEFAULT 'tabular',
                    layout_query JSON,
                    layout_options JSON,
                    refresh_interval INT,
                    filter JSON,
                    icon VARCHAR(64) DEFAULT 'bookmark',
                    color VARCHAR(255)
                )",
            sql_mysql: "
                CREATE TABLE IF NOT EXISTS nexus_settings (
                    id INT AUTO_INCREMENT PRIMARY KEY,
                    project_name VARCHAR(100) DEFAULT 'Nexus',
                    project_url VARCHAR(255),
                    project_color VARCHAR(16) DEFAULT '#6644FF',
                    project_logo CHAR(36),
                    public_foreground CHAR(36),
                    public_background CHAR(36),
                    public_note TEXT,
                    auth_login_attempts INT DEFAULT 25,
                    auth_password_policy VARCHAR(100),
                    storage_asset_transform VARCHAR(16) DEFAULT 'all',
                    storage_asset_presets JSON,
                    custom_css TEXT,
                    storage_default_folder CHAR(36),
                    basemaps JSON,
                    mapbox_key VARCHAR(255),
                    module_bar JSON,
                    project_descriptor VARCHAR(100),
                    default_language VARCHAR(16) DEFAULT 'en-US',
                    custom_aspect_ratios JSON,
                    public_favicon CHAR(36),
                    default_appearance VARCHAR(32) DEFAULT 'auto',
                    default_theme_light VARCHAR(255),
                    default_theme_dark VARCHAR(255),
                    theme_light_overrides JSON,
                    theme_dark_overrides JSON,
                    report_error_url VARCHAR(255),
                    report_bug_url VARCHAR(255),
                    report_feature_url VARCHAR(255)
                );
                CREATE TABLE IF NOT EXISTS nexus_presets (
                    id INT AUTO_INCREMENT PRIMARY KEY,
                    bookmark VARCHAR(255),
                    user CHAR(36),
                    role CHAR(36),
                    collection VARCHAR(64),
                    search VARCHAR(100),
                    layout VARCHAR(100) DEFAULT 'tabular',
                    layout_query JSON,
                    layout_options JSON,
                    refresh_interval INT,
                    filter JSON,
                    icon VARCHAR(64) DEFAULT 'bookmark',
                    color VARCHAR(255)
                )",
            sql_sqlite: "
                CREATE TABLE IF NOT EXISTS nexus_settings (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    project_name TEXT DEFAULT 'Nexus',
                    project_url TEXT,
                    project_color TEXT DEFAULT '#6644FF',
                    project_logo TEXT,
                    public_foreground TEXT,
                    public_background TEXT,
                    public_note TEXT,
                    auth_login_attempts INTEGER DEFAULT 25,
                    auth_password_policy TEXT,
                    storage_asset_transform TEXT DEFAULT 'all',
                    storage_asset_presets TEXT,
                    custom_css TEXT,
                    storage_default_folder TEXT,
                    basemaps TEXT,
                    mapbox_key TEXT,
                    module_bar TEXT,
                    project_descriptor TEXT,
                    default_language TEXT DEFAULT 'en-US',
                    custom_aspect_ratios TEXT,
                    public_favicon TEXT,
                    default_appearance TEXT DEFAULT 'auto',
                    default_theme_light TEXT,
                    default_theme_dark TEXT,
                    theme_light_overrides TEXT,
                    theme_dark_overrides TEXT,
                    report_error_url TEXT,
                    report_bug_url TEXT,
                    report_feature_url TEXT
                );
                CREATE TABLE IF NOT EXISTS nexus_presets (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    bookmark TEXT,
                    user TEXT REFERENCES nexus_users(id) ON DELETE CASCADE,
                    role TEXT REFERENCES nexus_roles(id) ON DELETE CASCADE,
                    collection TEXT,
                    search TEXT,
                    layout TEXT DEFAULT 'tabular',
                    layout_query TEXT,
                    layout_options TEXT,
                    refresh_interval INTEGER,
                    filter TEXT,
                    icon TEXT DEFAULT 'bookmark',
                    color TEXT
                )",
        },
        Migration {
            version: "20240101M".to_string(),
            name: "Create Dashboards, Panels, Notifications Tables".to_string(),
            sql_postgres: "
                CREATE TABLE IF NOT EXISTS nexus_dashboards (
                    id UUID PRIMARY KEY,
                    name VARCHAR(255) NOT NULL,
                    icon VARCHAR(64) DEFAULT 'dashboard',
                    note TEXT,
                    date_created TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                    user_created UUID REFERENCES nexus_users(id),
                    color VARCHAR(255)
                );
                CREATE TABLE IF NOT EXISTS nexus_panels (
                    id UUID PRIMARY KEY,
                    dashboard UUID NOT NULL REFERENCES nexus_dashboards(id) ON DELETE CASCADE,
                    name VARCHAR(255),
                    icon VARCHAR(64),
                    color VARCHAR(16),
                    show_header BOOLEAN DEFAULT FALSE,
                    note TEXT,
                    type VARCHAR(255) NOT NULL,
                    position_x INT NOT NULL DEFAULT 0,
                    position_y INT NOT NULL DEFAULT 0,
                    width INT NOT NULL DEFAULT 12,
                    height INT NOT NULL DEFAULT 8,
                    options JSON,
                    date_created TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                    user_created UUID REFERENCES nexus_users(id)
                );
                CREATE TABLE IF NOT EXISTS nexus_notifications (
                    id SERIAL PRIMARY KEY,
                    timestamp TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                    status VARCHAR(16) DEFAULT 'inbox',
                    recipient UUID NOT NULL REFERENCES nexus_users(id) ON DELETE CASCADE,
                    sender UUID REFERENCES nexus_users(id),
                    subject VARCHAR(255) NOT NULL,
                    message TEXT,
                    collection VARCHAR(64),
                    item VARCHAR(255)
                )",
            sql_mysql: "
                CREATE TABLE IF NOT EXISTS nexus_dashboards (
                    id CHAR(36) PRIMARY KEY,
                    name VARCHAR(255) NOT NULL,
                    icon VARCHAR(64) DEFAULT 'dashboard',
                    note TEXT,
                    date_created TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                    user_created CHAR(36),
                    color VARCHAR(255)
                );
                CREATE TABLE IF NOT EXISTS nexus_panels (
                    id CHAR(36) PRIMARY KEY,
                    dashboard CHAR(36) NOT NULL,
                    name VARCHAR(255),
                    icon VARCHAR(64),
                    color VARCHAR(16),
                    show_header BOOLEAN DEFAULT FALSE,
                    note TEXT,
                    type VARCHAR(255) NOT NULL,
                    position_x INT NOT NULL DEFAULT 0,
                    position_y INT NOT NULL DEFAULT 0,
                    width INT NOT NULL DEFAULT 12,
                    height INT NOT NULL DEFAULT 8,
                    options JSON,
                    date_created TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                    user_created CHAR(36)
                );
                CREATE TABLE IF NOT EXISTS nexus_notifications (
                    id INT AUTO_INCREMENT PRIMARY KEY,
                    timestamp TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                    status VARCHAR(16) DEFAULT 'inbox',
                    recipient CHAR(36) NOT NULL,
                    sender CHAR(36),
                    subject VARCHAR(255) NOT NULL,
                    message TEXT,
                    collection VARCHAR(64),
                    item VARCHAR(255)
                )",
            sql_sqlite: "
                CREATE TABLE IF NOT EXISTS nexus_dashboards (
                    id TEXT PRIMARY KEY,
                    name TEXT NOT NULL,
                    icon TEXT DEFAULT 'dashboard',
                    note TEXT,
                    date_created TEXT DEFAULT (datetime('now')),
                    user_created TEXT REFERENCES nexus_users(id),
                    color TEXT
                );
                CREATE TABLE IF NOT EXISTS nexus_panels (
                    id TEXT PRIMARY KEY,
                    dashboard TEXT NOT NULL REFERENCES nexus_dashboards(id) ON DELETE CASCADE,
                    name TEXT,
                    icon TEXT,
                    color TEXT,
                    show_header INTEGER DEFAULT 0,
                    note TEXT,
                    type TEXT NOT NULL,
                    position_x INTEGER NOT NULL DEFAULT 0,
                    position_y INTEGER NOT NULL DEFAULT 0,
                    width INTEGER NOT NULL DEFAULT 12,
                    height INTEGER NOT NULL DEFAULT 8,
                    options TEXT,
                    date_created TEXT DEFAULT (datetime('now')),
                    user_created TEXT REFERENCES nexus_users(id)
                );
                CREATE TABLE IF NOT EXISTS nexus_notifications (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    timestamp TEXT DEFAULT (datetime('now')),
                    status TEXT DEFAULT 'inbox',
                    recipient TEXT NOT NULL REFERENCES nexus_users(id) ON DELETE CASCADE,
                    sender TEXT REFERENCES nexus_users(id),
                    subject TEXT NOT NULL,
                    message TEXT,
                    collection TEXT,
                    item TEXT
                )",
        },
        Migration {
            version: "20240101N".to_string(),
            name: "Create Flows and Operations Tables".to_string(),
            sql_postgres: "
                CREATE TABLE IF NOT EXISTS nexus_flows (
                    id UUID PRIMARY KEY,
                    name VARCHAR(255) NOT NULL,
                    icon VARCHAR(64) DEFAULT 'bolt',
                    color VARCHAR(16),
                    description TEXT,
                    status VARCHAR(16) DEFAULT 'active',
                    trigger VARCHAR(255),
                    accountability VARCHAR(16) DEFAULT 'all',
                    options JSON,
                    operation UUID,
                    date_created TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                    user_created UUID REFERENCES nexus_users(id)
                );
                CREATE TABLE IF NOT EXISTS nexus_operations (
                    id UUID PRIMARY KEY,
                    name VARCHAR(255),
                    key VARCHAR(255) NOT NULL,
                    type VARCHAR(255) NOT NULL,
                    position_x INT NOT NULL DEFAULT 0,
                    position_y INT NOT NULL DEFAULT 0,
                    options JSON,
                    resolve UUID,
                    reject UUID,
                    flow UUID NOT NULL REFERENCES nexus_flows(id) ON DELETE CASCADE,
                    date_created TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                    user_created UUID REFERENCES nexus_users(id)
                )",
            sql_mysql: "
                CREATE TABLE IF NOT EXISTS nexus_flows (
                    id CHAR(36) PRIMARY KEY,
                    name VARCHAR(255) NOT NULL,
                    icon VARCHAR(64) DEFAULT 'bolt',
                    color VARCHAR(16),
                    description TEXT,
                    status VARCHAR(16) DEFAULT 'active',
                    `trigger` VARCHAR(255),
                    accountability VARCHAR(16) DEFAULT 'all',
                    options JSON,
                    operation CHAR(36),
                    date_created TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                    user_created CHAR(36)
                );
                CREATE TABLE IF NOT EXISTS nexus_operations (
                    id CHAR(36) PRIMARY KEY,
                    name VARCHAR(255),
                    `key` VARCHAR(255) NOT NULL,
                    type VARCHAR(255) NOT NULL,
                    position_x INT NOT NULL DEFAULT 0,
                    position_y INT NOT NULL DEFAULT 0,
                    options JSON,
                    resolve CHAR(36),
                    reject CHAR(36),
                    flow CHAR(36) NOT NULL,
                    date_created TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                    user_created CHAR(36)
                )",
            sql_sqlite: "
                CREATE TABLE IF NOT EXISTS nexus_flows (
                    id TEXT PRIMARY KEY,
                    name TEXT NOT NULL,
                    icon TEXT DEFAULT 'bolt',
                    color TEXT,
                    description TEXT,
                    status TEXT DEFAULT 'active',
                    trigger TEXT,
                    accountability TEXT DEFAULT 'all',
                    options TEXT,
                    operation TEXT,
                    date_created TEXT DEFAULT (datetime('now')),
                    user_created TEXT REFERENCES nexus_users(id)
                );
                CREATE TABLE IF NOT EXISTS nexus_operations (
                    id TEXT PRIMARY KEY,
                    name TEXT,
                    key TEXT NOT NULL,
                    type TEXT NOT NULL,
                    position_x INTEGER NOT NULL DEFAULT 0,
                    position_y INTEGER NOT NULL DEFAULT 0,
                    options TEXT,
                    resolve TEXT,
                    reject TEXT,
                    flow TEXT NOT NULL REFERENCES nexus_flows(id) ON DELETE CASCADE,
                    date_created TEXT DEFAULT (datetime('now')),
                    user_created TEXT REFERENCES nexus_users(id)
                )",
        },
        Migration {
            version: "20240101O".to_string(),
            name: "Create Webhooks, Shares, Translations, Versions, Comments Tables".to_string(),
            sql_postgres: "
                CREATE TABLE IF NOT EXISTS nexus_webhooks (
                    id SERIAL PRIMARY KEY,
                    name VARCHAR(255) NOT NULL,
                    method VARCHAR(10) DEFAULT 'POST',
                    url VARCHAR(255) NOT NULL,
                    status VARCHAR(16) DEFAULT 'active',
                    data BOOLEAN DEFAULT TRUE,
                    actions VARCHAR(100) NOT NULL,
                    collections VARCHAR(255) NOT NULL,
                    headers JSON,
                    was_active_before_deprecation BOOLEAN DEFAULT FALSE,
                    migrated_flow UUID
                );
                CREATE TABLE IF NOT EXISTS nexus_shares (
                    id UUID PRIMARY KEY,
                    name VARCHAR(255),
                    collection VARCHAR(64) NOT NULL,
                    item VARCHAR(255) NOT NULL,
                    role UUID REFERENCES nexus_roles(id),
                    password VARCHAR(255),
                    user_created UUID REFERENCES nexus_users(id),
                    date_created TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                    date_start TIMESTAMP,
                    date_end TIMESTAMP,
                    times_used INT DEFAULT 0,
                    max_uses INT
                );
                CREATE TABLE IF NOT EXISTS nexus_translations (
                    id UUID PRIMARY KEY,
                    language VARCHAR(16) NOT NULL,
                    key VARCHAR(255) NOT NULL,
                    value TEXT NOT NULL
                );
                CREATE TABLE IF NOT EXISTS nexus_versions (
                    id UUID PRIMARY KEY,
                    key VARCHAR(64) NOT NULL,
                    name VARCHAR(255),
                    collection VARCHAR(64) NOT NULL,
                    item VARCHAR(255) NOT NULL,
                    hash VARCHAR(255),
                    date_created TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                    date_updated TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                    user_created UUID REFERENCES nexus_users(id),
                    user_updated UUID REFERENCES nexus_users(id),
                    delta JSON
                );
                CREATE TABLE IF NOT EXISTS nexus_comments (
                    id UUID PRIMARY KEY,
                    collection VARCHAR(64) NOT NULL,
                    item VARCHAR(255) NOT NULL,
                    comment TEXT NOT NULL,
                    date_created TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                    date_updated TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                    user_created UUID REFERENCES nexus_users(id),
                    user_updated UUID REFERENCES nexus_users(id)
                )",
            sql_mysql: "
                CREATE TABLE IF NOT EXISTS nexus_webhooks (
                    id INT AUTO_INCREMENT PRIMARY KEY,
                    name VARCHAR(255) NOT NULL,
                    method VARCHAR(10) DEFAULT 'POST',
                    url VARCHAR(255) NOT NULL,
                    status VARCHAR(16) DEFAULT 'active',
                    data BOOLEAN DEFAULT TRUE,
                    actions VARCHAR(100) NOT NULL,
                    collections VARCHAR(255) NOT NULL,
                    headers JSON,
                    was_active_before_deprecation BOOLEAN DEFAULT FALSE,
                    migrated_flow CHAR(36)
                );
                CREATE TABLE IF NOT EXISTS nexus_shares (
                    id CHAR(36) PRIMARY KEY,
                    name VARCHAR(255),
                    collection VARCHAR(64) NOT NULL,
                    item VARCHAR(255) NOT NULL,
                    role CHAR(36),
                    password VARCHAR(255),
                    user_created CHAR(36),
                    date_created TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                    date_start TIMESTAMP NULL,
                    date_end TIMESTAMP NULL,
                    times_used INT DEFAULT 0,
                    max_uses INT
                );
                CREATE TABLE IF NOT EXISTS nexus_translations (
                    id CHAR(36) PRIMARY KEY,
                    language VARCHAR(16) NOT NULL,
                    `key` VARCHAR(255) NOT NULL,
                    value TEXT NOT NULL
                );
                CREATE TABLE IF NOT EXISTS nexus_versions (
                    id CHAR(36) PRIMARY KEY,
                    `key` VARCHAR(64) NOT NULL,
                    name VARCHAR(255),
                    collection VARCHAR(64) NOT NULL,
                    item VARCHAR(255) NOT NULL,
                    hash VARCHAR(255),
                    date_created TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                    date_updated TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
                    user_created CHAR(36),
                    user_updated CHAR(36),
                    delta JSON
                );
                CREATE TABLE IF NOT EXISTS nexus_comments (
                    id CHAR(36) PRIMARY KEY,
                    collection VARCHAR(64) NOT NULL,
                    item VARCHAR(255) NOT NULL,
                    comment TEXT NOT NULL,
                    date_created TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                    date_updated TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
                    user_created CHAR(36),
                    user_updated CHAR(36)
                )",
            sql_sqlite: "
                CREATE TABLE IF NOT EXISTS nexus_webhooks (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    name TEXT NOT NULL,
                    method TEXT DEFAULT 'POST',
                    url TEXT NOT NULL,
                    status TEXT DEFAULT 'active',
                    data INTEGER DEFAULT 1,
                    actions TEXT NOT NULL,
                    collections TEXT NOT NULL,
                    headers TEXT,
                    was_active_before_deprecation INTEGER DEFAULT 0,
                    migrated_flow TEXT
                );
                CREATE TABLE IF NOT EXISTS nexus_shares (
                    id TEXT PRIMARY KEY,
                    name TEXT,
                    collection TEXT NOT NULL,
                    item TEXT NOT NULL,
                    role TEXT REFERENCES nexus_roles(id),
                    password TEXT,
                    user_created TEXT REFERENCES nexus_users(id),
                    date_created TEXT DEFAULT (datetime('now')),
                    date_start TEXT,
                    date_end TEXT,
                    times_used INTEGER DEFAULT 0,
                    max_uses INTEGER
                );
                CREATE TABLE IF NOT EXISTS nexus_translations (
                    id TEXT PRIMARY KEY,
                    language TEXT NOT NULL,
                    key TEXT NOT NULL,
                    value TEXT NOT NULL
                );
                CREATE TABLE IF NOT EXISTS nexus_versions (
                    id TEXT PRIMARY KEY,
                    key TEXT NOT NULL,
                    name TEXT,
                    collection TEXT NOT NULL,
                    item TEXT NOT NULL,
                    hash TEXT,
                    date_created TEXT DEFAULT (datetime('now')),
                    date_updated TEXT DEFAULT (datetime('now')),
                    user_created TEXT REFERENCES nexus_users(id),
                    user_updated TEXT REFERENCES nexus_users(id),
                    delta TEXT
                );
                CREATE TABLE IF NOT EXISTS nexus_comments (
                    id TEXT PRIMARY KEY,
                    collection TEXT NOT NULL,
                    item TEXT NOT NULL,
                    comment TEXT NOT NULL,
                    date_created TEXT DEFAULT (datetime('now')),
                    date_updated TEXT DEFAULT (datetime('now')),
                    user_created TEXT REFERENCES nexus_users(id),
                    user_updated TEXT REFERENCES nexus_users(id)
                )",
        },
    ]
}
