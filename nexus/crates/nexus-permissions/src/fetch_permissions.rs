use crate::PermissionContext;
use nexus_database::SqlValue;
use nexus_types::accountability::Accountability;
use nexus_types::permissions::{Permission, PermissionsAction};
use serde_json::Value;

/// Options for fetching permissions
pub struct FetchPermissionsOptions<'a> {
    pub action: Option<PermissionsAction>,
    pub policies: &'a [String],
    pub collections: Option<&'a [String]>,
    pub accountability: Option<&'a Accountability>,
}

/// Fetch permissions for the given policies
/// Mirrors api/src/permissions/lib/fetch-permissions.ts
pub async fn fetch_permissions(
    options: FetchPermissionsOptions<'_>,
    ctx: &PermissionContext,
) -> Result<Vec<Permission>, Box<dyn std::error::Error + Send + Sync>> {
    if options.policies.is_empty() {
        return Ok(Vec::new());
    }

    let mut conditions = Vec::new();
    let mut bindings = Vec::new();
    let mut param_idx = 1;

    // Filter by policies
    let policy_placeholders: Vec<String> = options
        .policies
        .iter()
        .map(|_| {
            let p = format!("${}", param_idx);
            param_idx += 1;
            p
        })
        .collect();

    for policy in options.policies {
        bindings.push(SqlValue::Text(policy.clone()));
    }

    conditions.push(format!(
        "\"policy\" IN ({})",
        policy_placeholders.join(", ")
    ));

    // Filter by action
    if let Some(action) = options.action {
        let action_str = match action {
            PermissionsAction::Create => "create",
            PermissionsAction::Read => "read",
            PermissionsAction::Update => "update",
            PermissionsAction::Delete => "delete",
            PermissionsAction::Share => "share",
        };
        conditions.push(format!("\"action\" = ${}", param_idx));
        bindings.push(SqlValue::Text(action_str.to_string()));
        param_idx += 1;
    }

    // Filter by collections
    if let Some(collections) = options.collections {
        if !collections.is_empty() {
            let coll_placeholders: Vec<String> = collections
                .iter()
                .map(|_| {
                    let p = format!("${}", param_idx);
                    param_idx += 1;
                    p
                })
                .collect();

            for coll in collections {
                bindings.push(SqlValue::Text(coll.to_string()));
            }

            conditions.push(format!(
                "\"collection\" IN ({})",
                coll_placeholders.join(", ")
            ));
        }
    }

    let sql = format!(
        "SELECT \"id\", \"policy\", \"collection\", \"action\", \
         \"permissions\", \"validation\", \"presets\", \"fields\" \
         FROM \"directus_permissions\" \
         WHERE {}",
        conditions.join(" AND ")
    );

    let rows = ctx.db.query(&sql, &bindings).await?;

    let permissions: Vec<Permission> = rows
        .into_iter()
        .filter_map(|row| {
            let action_str = row.get("action")?.as_str()?;
            let action = match action_str {
                "create" => PermissionsAction::Create,
                "read" => PermissionsAction::Read,
                "update" => PermissionsAction::Update,
                "delete" => PermissionsAction::Delete,
                "share" => PermissionsAction::Share,
                _ => return None,
            };

            let fields = row
                .get("fields")
                .and_then(|v| v.as_str())
                .map(|s| s.split(',').map(|f| f.trim().to_string()).collect());

            let permissions_filter = row
                .get("permissions")
                .and_then(|v| {
                    if v.is_null() {
                        None
                    } else if let Some(s) = v.as_str() {
                        serde_json::from_str(s).ok()
                    } else {
                        serde_json::from_value(v.clone()).ok()
                    }
                });

            let validation = row
                .get("validation")
                .and_then(|v| {
                    if v.is_null() {
                        None
                    } else if let Some(s) = v.as_str() {
                        serde_json::from_str(s).ok()
                    } else {
                        serde_json::from_value(v.clone()).ok()
                    }
                });

            let presets = row
                .get("presets")
                .and_then(|v| {
                    if v.is_null() {
                        None
                    } else if let Some(s) = v.as_str() {
                        serde_json::from_str(s).ok()
                    } else {
                        serde_json::from_value(v.clone()).ok()
                    }
                });

            Some(Permission {
                id: row.get("id").and_then(|v| v.as_i64()),
                policy: row.get("policy").and_then(|v| v.as_str()).map(|s| s.to_string()),
                collection: row.get("collection")?.as_str()?.to_string(),
                action,
                permissions: permissions_filter,
                validation,
                presets,
                fields,
                system: None,
            })
        })
        .collect();

    Ok(permissions)
}
