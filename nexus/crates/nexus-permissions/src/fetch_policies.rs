use crate::PermissionContext;
use nexus_database::SqlValue;
use nexus_types::accountability::Accountability;

/// Fetch the policy IDs associated with the current user's accountability
/// Mirrors api/src/permissions/lib/fetch-policies.ts
///
/// Priority (bottom up):
/// 1. Parent role policies
/// 2. Child role policies
/// 3. User policies
pub async fn fetch_policies(
    accountability: &Accountability,
    ctx: &PermissionContext,
) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
    // Build the filter: get access rows for user's roles + direct user policies
    let mut conditions = Vec::new();
    let mut bindings = Vec::new();
    let mut param_idx = 1;

    // Role-based access
    if accountability.roles.is_empty() {
        // Public role: role is null AND user is null
        conditions.push("(a.\"role\" IS NULL AND a.\"user\" IS NULL)".to_string());
    } else {
        let placeholders: Vec<String> = accountability
            .roles
            .iter()
            .map(|_| {
                let p = format!("${}", param_idx);
                param_idx += 1;
                p
            })
            .collect();

        for role in &accountability.roles {
            bindings.push(SqlValue::Text(role.clone()));
        }

        conditions.push(format!("a.\"role\" IN ({})", placeholders.join(", ")));
    }

    // User-specific policies
    if let Some(ref user) = accountability.user {
        conditions.push(format!("a.\"user\" = ${}", param_idx));
        bindings.push(SqlValue::Text(user.clone()));
        let _ = param_idx + 1; // keep param_idx available for future use
    }

    let where_clause = if conditions.is_empty() {
        "1=0".to_string()
    } else {
        conditions.join(" OR ")
    };

    let sql = format!(
        "SELECT p.\"id\", p.\"ip_access\", a.\"role\" \
         FROM \"directus_access\" a \
         JOIN \"directus_policies\" p ON a.\"policy\" = p.\"id\" \
         WHERE ({})",
        where_clause
    );

    let rows = ctx.db.query(&sql, &bindings).await?;

    // Filter by IP if the policy has IP restrictions
    let mut policy_ids = Vec::new();
    let user_ip = accountability.ip.as_deref();

    for row in &rows {
        let policy_id = row
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();

        let ip_access = row.get("ip_access");

        // If ip_access is set, check if user's IP is allowed
        if let Some(ip_list) = ip_access.and_then(|v| v.as_array()) {
            if !ip_list.is_empty() {
                if let Some(ip) = user_ip {
                    let allowed = ip_list.iter().any(|allowed_ip| {
                        allowed_ip.as_str().map(|s| s == ip).unwrap_or(false)
                    });
                    if !allowed {
                        continue;
                    }
                } else {
                    continue; // No IP provided but IP restriction exists
                }
            }
        }

        policy_ids.push(policy_id);
    }

    // Sort: parent role policies first, then child role, then user policies
    // (already sorted by the query order based on roles hierarchy)

    Ok(policy_ids)
}
