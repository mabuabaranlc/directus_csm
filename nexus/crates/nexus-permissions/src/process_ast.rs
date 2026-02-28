//! Inject permission filters into the AST before execution.
//! Mirrors api/src/permissions/modules/process-ast/process-ast.ts
//!
//! For each node in the AST, look up the user's permissions for the
//! collection+action. If the permission has a filter, AND it into the
//! node's query filter. If the permission restricts fields, prune the
//! AST children to only allowed fields.

use crate::fetch_permissions::{fetch_permissions, FetchPermissionsOptions};
use crate::fetch_policies::fetch_policies;
use crate::PermissionContext;
use nexus_database::ast::{AstNode, ChildNode, RelationalNode};
use nexus_types::accountability::Accountability;
use nexus_types::filter::{Filter, LogicalFilter};
use nexus_types::permissions::{Permission, PermissionsAction};

#[derive(Debug, thiserror::Error)]
pub enum ProcessAstError {
    #[error("Access denied for collection '{collection}': {reason}")]
    AccessDenied { collection: String, reason: String },
    #[error("Permission lookup failed: {0}")]
    Lookup(String),
}

/// Process the AST by injecting permission filters and pruning restricted fields.
///
/// - Admin users: AST passes through unchanged.
/// - Non-admin users: Each collection node gets the permission's filter ANDed in,
///   and field lists are restricted to only allowed fields.
pub async fn process_ast(
    ast: &mut AstNode,
    accountability: &Accountability,
    action: PermissionsAction,
    ctx: &PermissionContext,
) -> Result<(), ProcessAstError> {
    // Admin bypass
    if accountability.admin {
        return Ok(());
    }

    // Fetch policies for this user
    let policies = fetch_policies(accountability, ctx)
        .await
        .map_err(|e| ProcessAstError::Lookup(e.to_string()))?;

    if policies.is_empty() {
        return Err(ProcessAstError::AccessDenied {
            collection: get_collection_name(ast),
            reason: "No policies found for user".into(),
        });
    }

    // Process the root node and all nested nodes recursively
    process_node(ast, action, &policies, ctx).await
}

fn process_node<'a>(
    node: &'a mut AstNode,
    action: PermissionsAction,
    policies: &'a [String],
    ctx: &'a PermissionContext,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), ProcessAstError>> + Send + 'a>> {
    Box::pin(async move {
        process_node_inner(node, action, policies, ctx).await
    })
}

async fn process_node_inner(
    node: &mut AstNode,
    action: PermissionsAction,
    policies: &[String],
    ctx: &PermissionContext,
) -> Result<(), ProcessAstError> {
    match node {
        AstNode::Root(ref mut root) => {
            inject_permissions_for_collection(
                &root.name,
                &mut root.query,
                &mut root.children,
                action,
                policies,
                ctx,
            )
            .await?;

            // Recurse into nested children
            for child in &mut root.children {
                if let ChildNode::Nested(ref mut nested) = child {
                    // Relational reads always use the Read action
                    process_node(nested, PermissionsAction::Read, policies, ctx).await?;
                }
            }
        }
        AstNode::M2O(ref mut rel) | AstNode::O2M(ref mut rel) | AstNode::A2O(ref mut rel) => {
            // For relational nodes, we need the collection name.
            // M2O: the related collection is implied by the relation setup.
            // We inject the read permissions for the related collection.
            let collection = get_relational_collection(rel);
            if let Some(ref coll) = collection {
                inject_permissions_for_collection(
                    coll,
                    &mut rel.query,
                    &mut rel.children,
                    PermissionsAction::Read,
                    policies,
                    ctx,
                )
                .await?;
            }

            // Recurse into nested children
            for child in &mut rel.children {
                if let ChildNode::Nested(ref mut nested) = child {
                    process_node(nested, PermissionsAction::Read, policies, ctx).await?;
                }
            }
        }
    }

    Ok(())
}

/// Inject permission filter and field restrictions for a specific collection.
async fn inject_permissions_for_collection(
    collection: &str,
    query: &mut nexus_types::query::Query,
    children: &mut Vec<ChildNode>,
    action: PermissionsAction,
    policies: &[String],
    ctx: &PermissionContext,
) -> Result<(), ProcessAstError> {
    let collections = vec![collection.to_string()];
    let permissions = fetch_permissions(
        FetchPermissionsOptions {
            action: Some(action),
            policies,
            collections: Some(&collections),
            accountability: None,
        },
        ctx,
    )
    .await
    .map_err(|e| ProcessAstError::Lookup(e.to_string()))?;

    // Find the matching permission
    let matching: Vec<&Permission> = permissions
        .iter()
        .filter(|p| p.collection == collection && p.action == action)
        .collect();

    if matching.is_empty() {
        return Err(ProcessAstError::AccessDenied {
            collection: collection.to_string(),
            reason: format!("No {:?} permission found", action),
        });
    }

    // Merge all matching permission filters with OR (multiple policies)
    let mut permission_filters: Vec<Filter> = Vec::new();
    let mut allowed_fields: Option<Vec<String>> = None;

    for perm in &matching {
        // Collect permission filters
        if let Some(ref filter) = perm.permissions {
            permission_filters.push(filter.clone());
        }

        // Merge allowed fields (union across policies)
        if let Some(ref fields) = perm.fields {
            if fields.contains(&"*".to_string()) {
                allowed_fields = None; // Full access
            } else if let Some(ref mut existing) = allowed_fields {
                for f in fields {
                    if !existing.contains(f) {
                        existing.push(f.clone());
                    }
                }
            } else if allowed_fields.is_none() {
                allowed_fields = Some(fields.clone());
            }
        }
    }

    // Inject permission filter into query
    if !permission_filters.is_empty() {
        let combined = if permission_filters.len() == 1 {
            permission_filters.into_iter().next().unwrap()
        } else {
            Filter::Logical(LogicalFilter::Or {
                _or: permission_filters,
            })
        };

        if let Some(existing) = query.filter.take() {
            // AND the existing filter with the permission filter
            query.filter = Some(Filter::Logical(LogicalFilter::And {
                _and: vec![existing, combined],
            }));
        } else {
            query.filter = Some(combined);
        }
    }

    // Prune children to allowed fields
    if let Some(ref allowed) = allowed_fields {
        children.retain(|child| match child {
            ChildNode::Field(f) => allowed.contains(&f.name),
            ChildNode::FunctionField(f) => allowed.contains(&f.name),
            ChildNode::Nested(n) => {
                // Keep nested if the relation field is allowed
                let name = match n.as_ref() {
                    AstNode::M2O(r) | AstNode::O2M(r) | AstNode::A2O(r) => &r.name,
                    AstNode::Root(r) => &r.name,
                };
                allowed.contains(name)
            }
        });
    }

    // Presets (if any) are applied at the service layer, not the AST layer.

    Ok(())
}

fn get_collection_name(node: &AstNode) -> String {
    match node {
        AstNode::Root(r) => r.name.clone(),
        AstNode::M2O(r) | AstNode::O2M(r) | AstNode::A2O(r) => r.name.clone(),
    }
}

fn get_relational_collection(rel: &RelationalNode) -> Option<String> {
    // The collection name for a relational node should be stored somewhere.
    // For now we use the name as a best guess. The AST builder should set this
    // properly based on the schema relations.
    Some(rel.name.clone())
}
