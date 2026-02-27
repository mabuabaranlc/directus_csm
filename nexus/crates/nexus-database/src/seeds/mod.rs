use crate::{DatabaseBackend, DatabaseError, SqlValue};
use std::sync::Arc;
use tracing;

/// Seed all system tables with default data
pub async fn seed_system_tables(db: &Arc<dyn DatabaseBackend>) -> Result<(), DatabaseError> {
    seed_default_role(db.as_ref()).await?;
    seed_default_settings(db.as_ref()).await?;
    Ok(())
}

/// Create the admin user with the given credentials
pub async fn create_admin_user(
    db: &Arc<dyn DatabaseBackend>,
    email: &str,
    password: &str,
) -> Result<(), DatabaseError> {
    // Hash the password with argon2
    let password_hash = hash_password(password)
        .map_err(|e| DatabaseError::Query(format!("Failed to hash password: {}", e)))?;

    let admin_role_id = get_or_create_admin_role(db.as_ref()).await?;
    let user_id = uuid::Uuid::new_v4().to_string();

    // Check if user with this email already exists
    let existing = db
        .query(
            "SELECT id FROM nexus_users WHERE email = $1",
            &[SqlValue::Text(email.to_string())],
        )
        .await?;

    if !existing.is_empty() {
        tracing::info!(email = %email, "Admin user already exists, skipping creation");
        return Ok(());
    }

    db.execute(
        "INSERT INTO nexus_users (id, email, password, first_name, last_name, role, status, provider) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        &[
            SqlValue::Text(user_id),
            SqlValue::Text(email.to_string()),
            SqlValue::Text(password_hash),
            SqlValue::Text("Admin".to_string()),
            SqlValue::Text("User".to_string()),
            SqlValue::Text(admin_role_id.clone()),
            SqlValue::Text("active".to_string()),
            SqlValue::Text("default".to_string()),
        ],
    )
    .await?;

    // Create admin policy and access entry
    let policy_id = get_or_create_admin_policy(db.as_ref()).await?;
    let access_id = uuid::Uuid::new_v4().to_string();

    // Check if access entry already exists for this role+policy
    let existing_access = db
        .query(
            "SELECT id FROM nexus_access WHERE role = $1 AND policy = $2",
            &[
                SqlValue::Text(admin_role_id.clone()),
                SqlValue::Text(policy_id.clone()),
            ],
        )
        .await?;

    if existing_access.is_empty() {
        db.execute(
            "INSERT INTO nexus_access (id, role, policy, sort) VALUES ($1, $2, $3, $4)",
            &[
                SqlValue::Text(access_id),
                SqlValue::Text(admin_role_id),
                SqlValue::Text(policy_id),
                SqlValue::Int(1),
            ],
        )
        .await?;
    }

    tracing::info!(email = %email, "Admin user created successfully");
    Ok(())
}

/// Seed the default Administrator role
async fn seed_default_role(db: &dyn DatabaseBackend) -> Result<(), DatabaseError> {
    let existing = db
        .query(
            "SELECT id FROM nexus_roles WHERE admin_access = $1",
            &[SqlValue::Bool(true)],
        )
        .await;

    match existing {
        Ok(rows) if rows.is_empty() => {
            let role_id = uuid::Uuid::new_v4().to_string();
            db.execute(
                "INSERT INTO nexus_roles (id, name, icon, description, admin_access, app_access) \
                 VALUES ($1, $2, $3, $4, $5, $6)",
                &[
                    SqlValue::Text(role_id),
                    SqlValue::Text("Administrator".to_string()),
                    SqlValue::Text("verified".to_string()),
                    SqlValue::Text("$t:admin_description".to_string()),
                    SqlValue::Bool(true),
                    SqlValue::Bool(true),
                ],
            )
            .await?;
            tracing::info!("Default Administrator role created");
        }
        Ok(_) => {
            tracing::debug!("Administrator role already exists");
        }
        Err(_) => {
            tracing::debug!("Roles table not yet available, skipping seed");
        }
    }

    Ok(())
}

/// Seed default settings
async fn seed_default_settings(db: &dyn DatabaseBackend) -> Result<(), DatabaseError> {
    let existing = db.query("SELECT id FROM nexus_settings LIMIT 1", &[]).await;

    match existing {
        Ok(rows) if rows.is_empty() => {
            db.execute(
                "INSERT INTO nexus_settings (project_name, project_color, default_language) \
                 VALUES ($1, $2, $3)",
                &[
                    SqlValue::Text("Nexus".to_string()),
                    SqlValue::Text("#6644FF".to_string()),
                    SqlValue::Text("en-US".to_string()),
                ],
            )
            .await?;
            tracing::info!("Default settings created");
        }
        Ok(_) => {
            tracing::debug!("Settings already exist");
        }
        Err(_) => {
            tracing::debug!("Settings table not yet available, skipping seed");
        }
    }

    Ok(())
}

/// Get or create the Administrator role, returning its ID
async fn get_or_create_admin_role(db: &dyn DatabaseBackend) -> Result<String, DatabaseError> {
    let rows = db
        .query(
            "SELECT id FROM nexus_roles WHERE admin_access = $1 LIMIT 1",
            &[SqlValue::Bool(true)],
        )
        .await?;

    if let Some(row) = rows.first() {
        if let Some(id) = row.get("id").and_then(|v| v.as_str()) {
            return Ok(id.to_string());
        }
    }

    let role_id = uuid::Uuid::new_v4().to_string();
    db.execute(
        "INSERT INTO nexus_roles (id, name, icon, description, admin_access, app_access) \
         VALUES ($1, $2, $3, $4, $5, $6)",
        &[
            SqlValue::Text(role_id.clone()),
            SqlValue::Text("Administrator".to_string()),
            SqlValue::Text("verified".to_string()),
            SqlValue::Text("$t:admin_description".to_string()),
            SqlValue::Bool(true),
            SqlValue::Bool(true),
        ],
    )
    .await?;

    Ok(role_id)
}

/// Get or create the Admin policy, returning its ID
async fn get_or_create_admin_policy(db: &dyn DatabaseBackend) -> Result<String, DatabaseError> {
    let rows = db
        .query(
            "SELECT id FROM nexus_policies WHERE admin_access = $1 LIMIT 1",
            &[SqlValue::Bool(true)],
        )
        .await?;

    if let Some(row) = rows.first() {
        if let Some(id) = row.get("id").and_then(|v| v.as_str()) {
            return Ok(id.to_string());
        }
    }

    let policy_id = uuid::Uuid::new_v4().to_string();
    db.execute(
        "INSERT INTO nexus_policies (id, name, icon, description, admin_access, app_access) \
         VALUES ($1, $2, $3, $4, $5, $6)",
        &[
            SqlValue::Text(policy_id.clone()),
            SqlValue::Text("Administrator".to_string()),
            SqlValue::Text("verified".to_string()),
            SqlValue::Text("$t:admin_policy_description".to_string()),
            SqlValue::Bool(true),
            SqlValue::Bool(true),
        ],
    )
    .await?;

    Ok(policy_id)
}

/// Hash a password using argon2
fn hash_password(password: &str) -> Result<String, String> {
    use argon2::{
        password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
        Argon2,
    };

    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| e.to_string())?
        .to_string();

    Ok(password_hash)
}
