use crate::PermissionContext;
use nexus_database::SqlValue;

/// Fetch the full role tree for a given role ID
/// Walks up the parent chain to build the complete roles list
/// Mirrors api/src/permissions/lib/fetch-roles-tree.ts
pub async fn fetch_roles_tree(
    role: Option<&str>,
    ctx: &PermissionContext,
) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
    let mut roles = Vec::new();

    let Some(start_role) = role else {
        return Ok(roles);
    };

    roles.push(start_role.to_string());

    let mut current_role = start_role.to_string();
    let mut seen = std::collections::HashSet::new();
    seen.insert(current_role.clone());

    loop {
        let sql = format!(
            "SELECT {} FROM {} WHERE {} = $1",
            ctx.db.quote_identifier("parent"),
            ctx.db.quote_identifier("directus_roles"),
            ctx.db.quote_identifier("id"),
        );

        let rows = ctx
            .db
            .query(&sql, &[SqlValue::Text(current_role.clone())])
            .await?;

        if let Some(row) = rows.first() {
            if let Some(parent) = row.get("parent").and_then(|v| v.as_str()) {
                if parent.is_empty() || seen.contains(parent) {
                    break; // No parent or circular reference
                }
                roles.push(parent.to_string());
                seen.insert(parent.to_string());
                current_role = parent.to_string();
            } else {
                break; // No parent
            }
        } else {
            break; // Role not found
        }
    }

    Ok(roles)
}
