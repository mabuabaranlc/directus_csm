//! Build an AST from a Query + SchemaOverview
//! Mirrors api/src/utils/get-ast-from-query/

use super::{AstNode, ChildNode, FieldNode, FunctionFieldNode, RelationalNode, RelationType, RootNode};
use nexus_types::query::Query;
use nexus_types::relations::Relation;
use nexus_types::schema::SchemaOverview;

#[derive(Debug, thiserror::Error)]
pub enum AstBuildError {
    #[error("Collection '{0}' not found in schema")]
    CollectionNotFound(String),
    #[error("Field '{0}' not found in collection '{1}'")]
    FieldNotFound(String, String),
    #[error("Relation not found for field '{0}' in collection '{1}'")]
    RelationNotFound(String, String),
}

/// Build an AST from a query, resolving relational fields from the schema.
///
/// Fields like `["id", "title", "author.name"]` are parsed:
///   - `id`, `title` → FieldNode
///   - `author.name` → look up relation for `author` → RelationalNode with FieldNode children
///   - `count(comments)` → FunctionFieldNode
///   - `*` → expand to all non-alias fields in the collection
pub fn get_ast_from_query(
    collection: &str,
    query: &Query,
    schema: &SchemaOverview,
) -> Result<AstNode, AstBuildError> {
    let _coll_overview = schema
        .collections
        .get(collection)
        .ok_or_else(|| AstBuildError::CollectionNotFound(collection.to_string()))?;

    let default_fields = vec!["*".to_string()];
    let raw_fields = query
        .fields
        .as_ref()
        .map(|f| f.as_slice())
        .unwrap_or(default_fields.as_slice());

    let children = parse_fields(collection, raw_fields, query, schema, &schema.relations)?;

    Ok(AstNode::Root(RootNode {
        name: collection.to_string(),
        children,
        query: query.clone(),
    }))
}

/// Parse a list of field strings into ChildNode variants.
fn parse_fields(
    collection: &str,
    fields: &[String],
    query: &Query,
    schema: &SchemaOverview,
    relations: &[Relation],
) -> Result<Vec<ChildNode>, AstBuildError> {
    let coll_overview = schema
        .collections
        .get(collection)
        .ok_or_else(|| AstBuildError::CollectionNotFound(collection.to_string()))?;

    let mut children = Vec::new();

    for field_str in fields {
        let field_str = field_str.trim();

        // Wildcard: expand to all concrete (non-alias) fields
        if field_str == "*" {
            for (name, fo) in &coll_overview.fields {
                if !fo.alias {
                    children.push(ChildNode::Field(FieldNode {
                        name: name.clone(),
                        alias: None,
                    }));
                }
            }
            // O2M relations are not auto-expanded on wildcard to avoid N+1 queries.
            // Users must explicitly request them (e.g., "comments.*").
            continue;
        }

        // Function fields: count(field), sum(field), etc.
        if let Some((func, inner)) = parse_function_field(field_str) {
            children.push(ChildNode::FunctionField(FunctionFieldNode {
                name: inner.to_string(),
                function: func.to_string(),
                alias: Some(format!("{}({})", func, inner)),
            }));
            continue;
        }

        // Dotted fields: "author.name" or "author.*"
        if let Some(dot_pos) = field_str.find('.') {
            let parent_field = &field_str[..dot_pos];
            let rest = &field_str[dot_pos + 1..];

            // Look up relation
            if let Some(rel_node) = resolve_relational_field(
                collection,
                parent_field,
                rest,
                query,
                schema,
                relations,
            )? {
                // Check if we already have a relational node for this parent
                let existing = children.iter_mut().find(|c| {
                    if let ChildNode::Nested(ref n) = c {
                        match n.as_ref() {
                            AstNode::M2O(r) | AstNode::O2M(r) | AstNode::A2O(r) => {
                                r.name == parent_field
                            }
                            _ => false,
                        }
                    } else {
                        false
                    }
                });

                if let Some(ChildNode::Nested(ref mut existing_node)) = existing {
                    // Merge children into existing relational node
                    let new_children = match rel_node {
                        AstNode::M2O(r) | AstNode::O2M(r) | AstNode::A2O(r) => r.children,
                        _ => Vec::new(),
                    };
                    match existing_node.as_mut() {
                        AstNode::M2O(ref mut r)
                        | AstNode::O2M(ref mut r)
                        | AstNode::A2O(ref mut r) => {
                            r.children.extend(new_children);
                        }
                        _ => {}
                    }
                } else {
                    children.push(ChildNode::Nested(Box::new(rel_node)));
                }
            }
            continue;
        }

        // Simple field
        let alias = query
            .alias
            .as_ref()
            .and_then(|a| a.get(field_str).cloned());

        children.push(ChildNode::Field(FieldNode {
            name: field_str.to_string(),
            alias,
        }));
    }

    Ok(children)
}

/// Resolve a dotted field reference to a relational AST node.
fn resolve_relational_field(
    collection: &str,
    field_name: &str,
    nested_fields: &str,
    query: &Query,
    schema: &SchemaOverview,
    relations: &[Relation],
) -> Result<Option<AstNode>, AstBuildError> {
    // Try M2O: current collection has a FK field pointing to another collection
    // e.g., articles.author_id → directus_users.id
    if let Some(rel) = relations.iter().find(|r| {
        r.collection == collection && r.field == field_name && r.related_collection.is_some()
    }) {
        let related = rel.related_collection.as_ref().unwrap();
        let related_coll = schema.collections.get(related.as_str());

        let nested: Vec<String> = if nested_fields == "*" {
            vec!["*".to_string()]
        } else {
            vec![nested_fields.to_string()]
        };

        let child_fields = parse_fields(related, &nested, &Default::default(), schema, relations)?;

        // For M2O: field_key is the FK column in the current table,
        // parent_key is the PK of the related table
        let parent_key = related_coll
            .map(|c| c.primary.clone())
            .unwrap_or_else(|| "id".to_string());

        return Ok(Some(AstNode::M2O(RelationalNode {
            name: field_name.to_string(),
            field_key: field_name.to_string(),
            parent_key,
            relation_type: RelationType::ManyToOne,
            children: child_fields,
            query: extract_deep_query(query, field_name),
        })));
    }

    // Try M2O with meta (field name differs from FK column)
    if let Some(rel) = relations.iter().find(|r| {
        if let Some(ref meta) = r.meta {
            r.collection == collection
                && meta.many_field == field_name
                && r.related_collection.is_some()
        } else {
            false
        }
    }) {
        let related = rel.related_collection.as_ref().unwrap();
        let related_coll = schema.collections.get(related.as_str());

        let nested: Vec<String> = if nested_fields == "*" {
            vec!["*".to_string()]
        } else {
            vec![nested_fields.to_string()]
        };

        let child_fields = parse_fields(related, &nested, &Default::default(), schema, relations)?;

        let parent_key = related_coll
            .map(|c| c.primary.clone())
            .unwrap_or_else(|| "id".to_string());

        return Ok(Some(AstNode::M2O(RelationalNode {
            name: field_name.to_string(),
            field_key: rel.field.clone(),
            parent_key,
            relation_type: RelationType::ManyToOne,
            children: child_fields,
            query: extract_deep_query(query, field_name),
        })));
    }

    // Try O2M: another collection has a FK pointing to this collection
    // e.g., comments.article_id → articles.id (field_name = "comments" on articles)
    if let Some(rel) = relations.iter().find(|r| {
        if let Some(ref meta) = r.meta {
            meta.one_collection.as_deref() == Some(collection)
                && meta.one_field.as_deref() == Some(field_name)
        } else {
            false
        }
    }) {
        let nested: Vec<String> = if nested_fields == "*" {
            vec!["*".to_string()]
        } else {
            vec![nested_fields.to_string()]
        };

        let child_fields = parse_fields(
            &rel.collection,
            &nested,
            &Default::default(),
            schema,
            relations,
        )?;

        let current_pk = schema
            .collections
            .get(collection)
            .map(|c| c.primary.clone())
            .unwrap_or_else(|| "id".to_string());

        return Ok(Some(AstNode::O2M(RelationalNode {
            name: field_name.to_string(),
            field_key: rel.field.clone(),       // FK in the child table
            parent_key: current_pk,             // PK of the parent table
            relation_type: RelationType::OneToMany,
            children: child_fields,
            query: extract_deep_query(query, field_name),
        })));
    }

    // Try A2O: many-to-one with collection discriminator field
    if let Some(rel) = relations.iter().find(|r| {
        if let Some(ref meta) = r.meta {
            r.collection == collection
                && meta.many_field == field_name
                && meta.one_collection_field.is_some()
        } else {
            false
        }
    }) {
        let meta = rel.meta.as_ref().unwrap();

        let nested: Vec<String> = if nested_fields == "*" {
            vec!["*".to_string()]
        } else {
            vec![nested_fields.to_string()]
        };

        // A2O can point to multiple collections. We'll resolve at runtime.
        let child_fields = if let Some(ref allowed) = meta.one_allowed_collections {
            if let Some(first) = allowed.first() {
                parse_fields(first, &nested, &Default::default(), schema, relations)
                    .unwrap_or_default()
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        return Ok(Some(AstNode::A2O(RelationalNode {
            name: field_name.to_string(),
            field_key: rel.field.clone(),
            parent_key: meta
                .one_collection_field
                .clone()
                .unwrap_or_else(|| "collection".to_string()),
            relation_type: RelationType::AnyToOne,
            children: child_fields,
            query: extract_deep_query(query, field_name),
        })));
    }

    // Not a relational field — could be a JSON path or invalid
    Ok(None)
}

/// Extract a deep query for a nested field from the parent query's `deep` parameter.
fn extract_deep_query(query: &Query, field_name: &str) -> Query {
    if let Some(ref deep) = query.deep {
        if let Some(sub) = deep.get(field_name) {
            if let Ok(dq) = serde_json::from_value::<nexus_types::query::DeepQuery>(sub.clone()) {
                return Query {
                    fields: dq._fields,
                    sort: dq._sort,
                    filter: dq._filter,
                    limit: dq._limit,
                    offset: dq._offset,
                    page: dq._page,
                    search: dq._search,
                    aggregate: dq._aggregate,
                    ..Default::default()
                };
            }
        }
    }
    Query::default()
}

/// Parse function field syntax like `count(comments)`, `sum(price)`
fn parse_function_field(s: &str) -> Option<(&str, &str)> {
    let open = s.find('(')?;
    let close = s.find(')')?;
    if close > open + 1 {
        let func = &s[..open];
        let inner = &s[open + 1..close];
        Some((func, inner))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexus_types::schema::{CollectionOverview, FieldOverview, SchemaOverview};
    use nexus_types::fields::FieldType;
    use nexus_types::relations::{Relation, RelationMeta};
    use std::collections::HashMap;

    fn test_schema() -> SchemaOverview {
        let mut articles_fields = HashMap::new();
        articles_fields.insert("id".into(), FieldOverview {
            field: "id".into(),
            default_value: None,
            nullable: false,
            generated: true,
            field_type: FieldType::Uuid,
            db_type: None,
            precision: None,
            scale: None,
            special: vec![],
            note: None,
            validation: None,
            alias: false,
            searchable: false,
        });
        articles_fields.insert("title".into(), FieldOverview {
            field: "title".into(),
            default_value: None,
            nullable: false,
            generated: false,
            field_type: FieldType::String,
            db_type: None,
            precision: None,
            scale: None,
            special: vec![],
            note: None,
            validation: None,
            alias: false,
            searchable: true,
        });
        articles_fields.insert("author".into(), FieldOverview {
            field: "author".into(),
            default_value: None,
            nullable: true,
            generated: false,
            field_type: FieldType::Uuid,
            db_type: None,
            precision: None,
            scale: None,
            special: vec![],
            note: None,
            validation: None,
            alias: false,
            searchable: false,
        });

        let mut users_fields = HashMap::new();
        users_fields.insert("id".into(), FieldOverview {
            field: "id".into(),
            default_value: None,
            nullable: false,
            generated: true,
            field_type: FieldType::Uuid,
            db_type: None,
            precision: None,
            scale: None,
            special: vec![],
            note: None,
            validation: None,
            alias: false,
            searchable: false,
        });
        users_fields.insert("name".into(), FieldOverview {
            field: "name".into(),
            default_value: None,
            nullable: false,
            generated: false,
            field_type: FieldType::String,
            db_type: None,
            precision: None,
            scale: None,
            special: vec![],
            note: None,
            validation: None,
            alias: false,
            searchable: true,
        });

        let mut collections = HashMap::new();
        collections.insert("articles".into(), CollectionOverview {
            collection: "articles".into(),
            primary: "id".into(),
            fields: articles_fields,
            is_singleton: false,
            accountability: Some("all".into()),
        });
        collections.insert("users".into(), CollectionOverview {
            collection: "users".into(),
            primary: "id".into(),
            fields: users_fields,
            is_singleton: false,
            accountability: Some("all".into()),
        });

        let relations = vec![Relation {
            collection: "articles".into(),
            field: "author".into(),
            related_collection: Some("users".into()),
            schema: None,
            meta: Some(RelationMeta {
                id: None,
                many_collection: "articles".into(),
                many_field: "author".into(),
                one_collection: Some("users".into()),
                one_field: None,
                one_collection_field: None,
                one_allowed_collections: None,
                one_deselect_action: "nullify".into(),
                junction_field: None,
                sort_field: None,
                system: None,
            }),
        }];

        SchemaOverview { collections, relations }
    }

    #[test]
    fn test_flat_fields() {
        let schema = test_schema();
        let query = Query {
            fields: Some(vec!["id".into(), "title".into()]),
            ..Default::default()
        };
        let ast = get_ast_from_query("articles", &query, &schema).unwrap();
        match ast {
            AstNode::Root(root) => {
                assert_eq!(root.children.len(), 2);
            }
            _ => panic!("Expected Root"),
        }
    }

    #[test]
    fn test_m2o_field() {
        let schema = test_schema();
        let query = Query {
            fields: Some(vec!["id".into(), "author.name".into()]),
            ..Default::default()
        };
        let ast = get_ast_from_query("articles", &query, &schema).unwrap();
        match ast {
            AstNode::Root(root) => {
                assert_eq!(root.children.len(), 2);
                let nested = root.children.iter().find(|c| matches!(c, ChildNode::Nested(_)));
                assert!(nested.is_some());
            }
            _ => panic!("Expected Root"),
        }
    }

    #[test]
    fn test_wildcard() {
        let schema = test_schema();
        let query = Query {
            fields: Some(vec!["*".into()]),
            ..Default::default()
        };
        let ast = get_ast_from_query("articles", &query, &schema).unwrap();
        match ast {
            AstNode::Root(root) => {
                // Should have all non-alias fields: id, title, author
                assert!(root.children.len() >= 3);
            }
            _ => panic!("Expected Root"),
        }
    }
}
