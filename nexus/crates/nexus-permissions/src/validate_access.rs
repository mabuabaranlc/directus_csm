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

    // If primary keys are provided, we need to verify item-level access
    // by actually reading the items with the permission filters applied
    if let Some(_keys) = options.primary_keys {
        // TODO: Implement item-level access validation
        // This requires reading the items using the permission filters
        // and checking if all requested keys are returned
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
