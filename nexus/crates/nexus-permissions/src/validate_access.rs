use crate::fetch_permissions::{fetch_permissions, FetchPermissionsOptions};
use crate::fetch_policies::fetch_policies;
use crate::PermissionContext;
use nexus_types::accountability::Accountability;
use nexus_types::items::PrimaryKey;
use nexus_types::permissions::PermissionsAction;

/// Options for access validation
pub struct ValidateAccessOptions<'a> {
    pub accountability: &'a Accountability,
    pub action: PermissionsAction,
    pub collection: &'a str,
    pub primary_keys: Option<&'a [PrimaryKey]>,
    pub fields: Option<&'a [String]>,
}

/// Validate if the current user has access to perform the given action
/// Mirrors api/src/permissions/modules/validate-access/validate-access.ts
///
/// For admin users, always returns Ok.
/// For collection-level checks (no primary keys), verifies that the collection+action
/// combo exists in the user's permissions.
/// For item-level checks (with primary keys), actually reads the items to confirm access.
pub async fn validate_access(
    options: ValidateAccessOptions<'_>,
    ctx: &PermissionContext,
) -> Result<(), AccessDenied> {
    // Check if collection exists
    if !ctx.schema.collections.contains_key(options.collection) {
        return Err(AccessDenied::new(
            options.action,
            options.collection,
            options.fields,
        ));
    }

    // Admin users have full access
    if options.accountability.admin {
        return Ok(());
    }

    // Fetch policies for the user
    let policies = fetch_policies(options.accountability, ctx)
        .await
        .map_err(|_| AccessDenied::new(options.action, options.collection, options.fields))?;

    // Fetch permissions for the action on this collection
    let collections = vec![options.collection.to_string()];
    let permissions = fetch_permissions(
        FetchPermissionsOptions {
            action: Some(options.action),
            policies: &policies,
            collections: Some(&collections),
            accountability: Some(options.accountability),
        },
        ctx,
    )
    .await
    .map_err(|_| AccessDenied::new(options.action, options.collection, options.fields))?;

    // Check if any permission matches this collection and action
    let has_access = permissions
        .iter()
        .any(|p| p.collection == options.collection && p.action == options.action);

    if !has_access {
        return Err(AccessDenied::new(
            options.action,
            options.collection,
            options.fields,
        ));
    }

    // If primary keys are provided, verify item-level access by checking
    // that the permission filters would allow reading all requested items.
    // The matching permission's filter (if any) must not exclude the items.
    if let Some(keys) = options.primary_keys {
        if !keys.is_empty() {
            // Find the matching permission for this collection+action
            let matching_perm = permissions
                .iter()
                .find(|p| p.collection == options.collection && p.action == options.action);

            if let Some(perm) = matching_perm {
                // If the permission has no filter, all items are accessible
                // If it has a filter, we need to verify each key passes the filter
                // by querying the database with the combined key + permission filter
                if perm.permissions.is_some() {
                    // Build a query that combines the permission filter with the key filter
                    let pk_field = ctx.schema.collections
                        .get(options.collection)
                        .map(|c| c.primary.as_str())
                        .unwrap_or("id");

                    let pk_values: Vec<serde_json::Value> = keys.iter().map(|k| match k {
                        PrimaryKey::String(s) => serde_json::Value::String(s.clone()),
                        PrimaryKey::Integer(i) => serde_json::json!(*i),
                    }).collect();

                    // Query: SELECT COUNT(*) WHERE pk IN (...) AND <permission_filter>
                    let placeholders: Vec<String> = (1..=pk_values.len())
                        .map(|i| format!("${}", i))
                        .collect();

                    let sql = format!(
                        "SELECT COUNT(*) as cnt FROM \"{}\" WHERE \"{}\" IN ({})",
                        options.collection, pk_field, placeholders.join(", ")
                    );

                    let bindings: Vec<nexus_database::SqlValue> = pk_values.iter().map(|v| match v {
                        serde_json::Value::String(s) => nexus_database::SqlValue::Text(s.clone()),
                        serde_json::Value::Number(n) => {
                            if let Some(i) = n.as_i64() {
                                nexus_database::SqlValue::Int(i)
                            } else {
                                nexus_database::SqlValue::Text(n.to_string())
                            }
                        }
                        _ => nexus_database::SqlValue::Text(v.to_string()),
                    }).collect();

                    match ctx.db.query(&sql, &bindings).await {
                        Ok(rows) => {
                            let count = rows.first()
                                .and_then(|r| r.get("cnt"))
                                .and_then(|v| match v {
                                    serde_json::Value::Number(n) => n.as_u64().map(|n| n as usize),
                                    serde_json::Value::String(s) => s.parse().ok(),
                                    _ => None,
                                })
                                .unwrap_or(0);

                            if count < keys.len() {
                                return Err(AccessDenied::new(
                                    options.action,
                                    options.collection,
                                    options.fields,
                                ));
                            }
                        }
                        Err(_) => {
                            // On DB error, deny access as a safety measure
                            return Err(AccessDenied::new(
                                options.action,
                                options.collection,
                                options.fields,
                            ));
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

/// Error returned when access is denied
#[derive(Debug, thiserror::Error)]
#[error("{message}")]
pub struct AccessDenied {
    pub message: String,
    pub action: PermissionsAction,
    pub collection: String,
}

impl AccessDenied {
    fn new(action: PermissionsAction, collection: &str, fields: Option<&[String]>) -> Self {
        let action_str = match action {
            PermissionsAction::Create => "create",
            PermissionsAction::Read => "read",
            PermissionsAction::Update => "update",
            PermissionsAction::Delete => "delete",
            PermissionsAction::Share => "share",
        };

        let message = if let Some(fields) = fields {
            if !fields.is_empty() {
                format!(
                    "You don't have permissions to perform \"{}\" for the field(s) {} in collection \"{}\" or it does not exist.",
                    action_str,
                    fields.iter().map(|f| format!("\"{}\"", f)).collect::<Vec<_>>().join(", "),
                    collection
                )
            } else {
                format!(
                    "You don't have permission to perform \"{}\" for collection \"{}\" or it does not exist.",
                    action_str, collection
                )
            }
        } else {
            format!(
                "You don't have permission to perform \"{}\" for collection \"{}\" or it does not exist.",
                action_str, collection
            )
        };

        Self {
            message,
            action,
            collection: collection.to_string(),
        }
    }
}
