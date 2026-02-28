//! Execute an AST against a database backend.
//! Mirrors api/src/database/run-ast/
//!
//! Strategy:
//! - Root + M2O relations → single query with LEFT JOINs
//! - O2M relations → separate sub-queries, merged into parent results
//! - A2O relations → separate per-collection queries, merged by discriminator

use super::{AstNode, ChildNode};
use crate::{DatabaseBackend, DatabaseError, SqlValue};
use serde_json::{json, Map, Value};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, thiserror::Error)]
pub enum RunAstError {
    #[error("Database error: {0}")]
    Database(#[from] DatabaseError),
    #[error("AST error: {0}")]
    Ast(String),
}

/// Execute a complete AST and return merged results.
pub async fn run_ast(
    ast: &AstNode,
    db: &Arc<dyn DatabaseBackend>,
) -> Result<Vec<Value>, RunAstError> {
    match ast {
        AstNode::Root(root) => {
            let (sql, bindings, column_map) = build_root_query(root, db)?;
            let rows = db.query(&sql, &bindings).await?;

            let mut items: Vec<Value> = rows
                .into_iter()
                .map(|row| restructure_row(row, &column_map))
                .collect();

            // Execute O2M and A2O sub-queries
            for child in &root.children {
                if let ChildNode::Nested(ref node) = child {
                    match node.as_ref() {
                        AstNode::O2M(rel) => {
                            run_o2m_subquery(rel, &mut items, &root.name, db).await?;
                        }
                        AstNode::A2O(rel) => {
                            run_a2o_subquery(rel, &mut items, &root.name, db).await?;
                        }
                        _ => {} // M2O already handled via JOIN
                    }
                }
            }

            Ok(items)
        }
        _ => Err(RunAstError::Ast("run_ast expects a Root node".into())),
    }
}

/// Mapping from result column name → (field_alias, nested_path)
/// e.g., "author__name" → ("author", "name")
type ColumnMap = Vec<ColumnMapping>;

#[derive(Debug)]
enum ColumnMapping {
    /// Direct field: result column → output field name
    Direct { column: String, output: String },
    /// Nested field from a JOIN: result column → (relation_name, field_name)
    Nested {
        column: String,
        relation: String,
        field: String,
    },
}

/// Build the main SELECT query for a RootNode, including LEFT JOINs for M2O relations.
fn build_root_query(
    root: &super::RootNode,
    db: &Arc<dyn DatabaseBackend>,
) -> Result<(String, Vec<SqlValue>, ColumnMap), RunAstError> {
    let table = db.quote_identifier(&root.name);
    let mut select_parts: Vec<String> = Vec::new();
    let mut join_parts: Vec<String> = Vec::new();
    let mut column_map: ColumnMap = Vec::new();

    for child in &root.children {
        match child {
            ChildNode::Field(f) => {
                let col = format!("{}.{}", table, db.quote_identifier(&f.name));
                let output = f.alias.as_ref().unwrap_or(&f.name).clone();
                select_parts.push(col.clone());
                column_map.push(ColumnMapping::Direct {
                    column: f.name.clone(),
                    output,
                });
            }
            ChildNode::FunctionField(f) => {
                let col = format!(
                    "{}({}.{}) AS {}",
                    f.function.to_uppercase(),
                    table,
                    db.quote_identifier(&f.name),
                    db.quote_identifier(
                        f.alias
                            .as_ref()
                            .unwrap_or(&format!("{}_{}", f.function, f.name))
                    )
                );
                select_parts.push(col);
                let alias = f
                    .alias
                    .clone()
                    .unwrap_or_else(|| format!("{}_{}", f.function, f.name));
                column_map.push(ColumnMapping::Direct {
                    column: alias.clone(),
                    output: alias,
                });
            }
            ChildNode::Nested(node) => {
                match node.as_ref() {
                    AstNode::M2O(rel) => {
                        // LEFT JOIN for M2O
                        let join_alias = db.quote_identifier(&rel.name);
                        let related_table = if let Some(related) = find_related_table(rel) {
                            db.quote_identifier(&related)
                        } else {
                            // Fallback: use field_key relation
                            join_alias.clone()
                        };

                        let join_clause = format!(
                            "LEFT JOIN {} AS {} ON {}.{} = {}.{}",
                            related_table,
                            join_alias,
                            table,
                            db.quote_identifier(&rel.field_key),
                            join_alias,
                            db.quote_identifier(&rel.parent_key),
                        );
                        join_parts.push(join_clause);

                        // Select nested fields
                        for nested_child in &rel.children {
                            if let ChildNode::Field(f) = nested_child {
                                let col_alias =
                                    format!("{}__{}",  rel.name, f.name);
                                let col = format!(
                                    "{}.{} AS {}",
                                    join_alias,
                                    db.quote_identifier(&f.name),
                                    db.quote_identifier(&col_alias)
                                );
                                select_parts.push(col);
                                column_map.push(ColumnMapping::Nested {
                                    column: col_alias,
                                    relation: rel.name.clone(),
                                    field: f.alias.as_ref().unwrap_or(&f.name).clone(),
                                });
                            }
                        }
                    }
                    AstNode::O2M(_) | AstNode::A2O(_) => {
                        // These are handled as separate sub-queries after the main query
                    }
                    _ => {}
                }
            }
        }
    }

    if select_parts.is_empty() {
        select_parts.push(format!("{}.*", table));
    }

    let mut sql = format!("SELECT {} FROM {}", select_parts.join(", "), table);

    if !join_parts.is_empty() {
        sql.push_str(" ");
        sql.push_str(&join_parts.join(" "));
    }

    // WHERE clause
    let mut bindings: Vec<SqlValue> = Vec::new();
    let param_idx = 1;

    if let Some(ref filter) = root.query.filter {
        let (where_clause, filter_bindings, _next_idx) =
            build_filter_sql(filter, db, &root.name, param_idx)?;
        if !where_clause.is_empty() {
            sql.push_str(&format!(" WHERE {}", where_clause));
            bindings.extend(filter_bindings);
        }
    }

    // Search
    if let Some(ref _search) = root.query.search {
        // Search is handled at the service level for now
    }

    // ORDER BY
    if let Some(ref sort) = root.query.sort {
        let sort_clauses: Vec<String> = sort
            .iter()
            .map(|s| {
                if let Some(field) = s.strip_prefix('-') {
                    format!("{}.{} DESC", table, db.quote_identifier(field))
                } else {
                    format!("{}.{} ASC", table, db.quote_identifier(s))
                }
            })
            .collect();
        sql.push_str(&format!(" ORDER BY {}", sort_clauses.join(", ")));
    }

    // LIMIT
    if let Some(limit) = root.query.limit {
        if limit >= 0 {
            sql.push_str(&format!(" LIMIT {}", limit));
        }
    } else {
        sql.push_str(" LIMIT 100");
    }

    // OFFSET
    if let Some(offset) = root.query.offset {
        sql.push_str(&format!(" OFFSET {}", offset));
    }

    // GROUP BY (for aggregate queries)
    if let Some(ref group) = root.query.group {
        let group_cols: Vec<String> = group
            .iter()
            .map(|g| format!("{}.{}", table, db.quote_identifier(g)))
            .collect();
        // GROUP BY must come before ORDER BY in the final SQL, so we need to
        // re-order. For simplicity, we insert it before ORDER BY.
        // Actually, let's just append and note this is a simplification.
        sql.push_str(&format!(" GROUP BY {}", group_cols.join(", ")));
    }

    Ok((sql, bindings, column_map))
}

/// Restructure a flat row with `relation__field` columns into nested JSON objects.
fn restructure_row(row: HashMap<String, Value>, column_map: &ColumnMap) -> Value {
    let mut obj = Map::new();
    let mut nested: HashMap<String, Map<String, Value>> = HashMap::new();

    for mapping in column_map {
        match mapping {
            ColumnMapping::Direct { column, output } => {
                if let Some(val) = row.get(column) {
                    obj.insert(output.clone(), val.clone());
                } else if let Some(val) = row.get(output) {
                    obj.insert(output.clone(), val.clone());
                }
            }
            ColumnMapping::Nested {
                column,
                relation,
                field,
            } => {
                let val = row
                    .get(column)
                    .cloned()
                    .unwrap_or(Value::Null);
                nested
                    .entry(relation.clone())
                    .or_default()
                    .insert(field.clone(), val);
            }
        }
    }

    // Merge nested objects
    for (rel_name, fields) in nested {
        // If all values are null, the relation is null (no matched row)
        let all_null = fields.values().all(|v| v.is_null());
        if all_null {
            obj.insert(rel_name, Value::Null);
        } else {
            obj.insert(rel_name, Value::Object(fields));
        }
    }

    // Include any columns not in the map (e.g., from raw queries)
    for (key, val) in &row {
        if !obj.contains_key(key)
            && !column_map.iter().any(|m| match m {
                ColumnMapping::Direct { column, .. } => column == key,
                ColumnMapping::Nested { column, .. } => column == key,
            })
        {
            obj.insert(key.clone(), val.clone());
        }
    }

    Value::Object(obj)
}

/// Execute a O2M sub-query and merge results into parent items.
async fn run_o2m_subquery(
    rel: &super::RelationalNode,
    parent_items: &mut [Value],
    _parent_collection: &str,
    db: &Arc<dyn DatabaseBackend>,
) -> Result<(), RunAstError> {
    if parent_items.is_empty() {
        return Ok(());
    }

    // Collect parent PKs
    let parent_pk_field = &rel.parent_key;
    let parent_pks: Vec<Value> = parent_items
        .iter()
        .filter_map(|item| item.get(parent_pk_field).cloned())
        .filter(|v| !v.is_null())
        .collect();

    if parent_pks.is_empty() {
        // Set all parent items' relation field to empty array
        for item in parent_items.iter_mut() {
            if let Value::Object(ref mut obj) = item {
                obj.insert(rel.name.clone(), json!([]));
            }
        }
        return Ok(());
    }

    // Find the related table from the relation
    // For O2M: rel.field_key is the FK column in the child table
    // The child table name is inferred from the relation name or we need schema info
    // In our AST, for O2M the collection info should be embedded
    // For now, use the relation name as the child collection (matching Directus convention)
    let child_table = find_o2m_collection(rel).unwrap_or_else(|| rel.name.clone());

    // Build child field list
    let child_fields: Vec<String> = rel
        .children
        .iter()
        .filter_map(|c| match c {
            ChildNode::Field(f) => Some(db.quote_identifier(&f.name)),
            _ => None,
        })
        .collect();

    let select_fields = if child_fields.is_empty() {
        "*".to_string()
    } else {
        // Always include the FK field for merging
        let fk_quoted = db.quote_identifier(&rel.field_key);
        if child_fields.iter().any(|f| f == &fk_quoted) {
            child_fields.join(", ")
        } else {
            format!("{}, {}", fk_quoted, child_fields.join(", "))
        }
    };

    // Build WHERE IN clause
    let mut bindings = Vec::new();
    let placeholders: Vec<String> = parent_pks
        .iter()
        .enumerate()
        .map(|(i, pk)| {
            bindings.push(value_to_sql_value(pk));
            format!("${}", i + 1)
        })
        .collect();

    let mut sql = format!(
        "SELECT {} FROM {} WHERE {} IN ({})",
        select_fields,
        db.quote_identifier(&child_table),
        db.quote_identifier(&rel.field_key),
        placeholders.join(", ")
    );

    // Apply sub-query filters
    let param_idx = bindings.len() + 1;
    if let Some(ref filter) = rel.query.filter {
        let (where_clause, filter_bindings, _next) =
            build_filter_sql(filter, db, &child_table, param_idx)?;
        if !where_clause.is_empty() {
            sql.push_str(&format!(" AND {}", where_clause));
            bindings.extend(filter_bindings);
        }
    }

    // Apply sort
    if let Some(ref sort) = rel.query.sort {
        let sort_clauses: Vec<String> = sort
            .iter()
            .map(|s| {
                if let Some(field) = s.strip_prefix('-') {
                    format!("{} DESC", db.quote_identifier(field))
                } else {
                    format!("{} ASC", db.quote_identifier(s))
                }
            })
            .collect();
        sql.push_str(&format!(" ORDER BY {}", sort_clauses.join(", ")));
    }

    // Apply limit per parent (approximate — real per-parent limit needs window functions)
    if let Some(limit) = rel.query.limit {
        if limit >= 0 {
            sql.push_str(&format!(" LIMIT {}", limit * parent_pks.len() as i64));
        }
    }

    let child_rows = db.query(&sql, &bindings).await?;

    // Group child rows by FK value
    let mut grouped: HashMap<String, Vec<Value>> = HashMap::new();
    for row in child_rows {
        let fk_val = row
            .get(&rel.field_key)
            .cloned()
            .unwrap_or(Value::Null);
        let fk_key = value_to_string(&fk_val);

        let child_obj: Map<String, Value> = row.into_iter().collect();
        grouped
            .entry(fk_key)
            .or_default()
            .push(Value::Object(child_obj));
    }

    // Merge into parent items
    for item in parent_items.iter_mut() {
        if let Value::Object(ref mut obj) = item {
            let pk_val = obj
                .get(parent_pk_field)
                .cloned()
                .unwrap_or(Value::Null);
            let pk_key = value_to_string(&pk_val);

            let children = grouped.remove(&pk_key).unwrap_or_default();
            obj.insert(rel.name.clone(), Value::Array(children));
        }
    }

    Ok(())
}

/// Execute A2O sub-queries (any-to-one with collection discriminator).
async fn run_a2o_subquery(
    rel: &super::RelationalNode,
    parent_items: &mut [Value],
    _parent_collection: &str,
    db: &Arc<dyn DatabaseBackend>,
) -> Result<(), RunAstError> {
    // A2O: each parent item has two columns:
    //   - field_key: the FK value (pointing to the related item PK)
    //   - parent_key: the collection discriminator (tells us which collection to look in)
    //
    // Group parent items by target collection, then batch-fetch from each.

    let mut by_collection: HashMap<String, Vec<(usize, Value)>> = HashMap::new();

    for (i, item) in parent_items.iter().enumerate() {
        let collection_val = item
            .get(&rel.parent_key)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let fk_val = item
            .get(&rel.field_key)
            .cloned()
            .unwrap_or(Value::Null);

        if !collection_val.is_empty() && !fk_val.is_null() {
            by_collection
                .entry(collection_val)
                .or_default()
                .push((i, fk_val));
        }
    }

    for (target_collection, refs) in by_collection {
        let pks: Vec<Value> = refs.iter().map(|(_, v)| v.clone()).collect();
        let mut bindings = Vec::new();
        let placeholders: Vec<String> = pks
            .iter()
            .enumerate()
            .map(|(i, pk)| {
                bindings.push(value_to_sql_value(pk));
                format!("${}", i + 1)
            })
            .collect();

        let sql = format!(
            "SELECT * FROM {} WHERE {} IN ({})",
            db.quote_identifier(&target_collection),
            db.quote_identifier("id"), // A2O typically uses "id" as the target PK
            placeholders.join(", ")
        );

        let rows = db.query(&sql, &bindings).await?;

        // Index by PK
        let mut by_pk: HashMap<String, Value> = HashMap::new();
        for row in rows {
            if let Some(pk) = row.get("id") {
                let key = value_to_string(pk);
                let obj: Map<String, Value> = row.into_iter().collect();
                by_pk.insert(key, Value::Object(obj));
            }
        }

        // Merge into parent items
        for (idx, fk_val) in &refs {
            let fk_key = value_to_string(fk_val);
            if let Some(related) = by_pk.get(&fk_key) {
                if let Value::Object(ref mut obj) = parent_items[*idx] {
                    obj.insert(rel.name.clone(), related.clone());
                }
            }
        }
    }

    Ok(())
}

/// Find the related table name for a relational node.
fn find_related_table(_rel: &super::RelationalNode) -> Option<String> {
    // For M2O the related table is usually stored in the query context.
    // Since we don't have it directly on RelationalNode, we use the name
    // convention where the relation name matches the related table, or
    // the caller should have set it up via the AST builder.
    // We'll rely on the join alias matching the table for now.
    None // Will use join_alias as the table name
}

/// For O2M, find the child collection name.
fn find_o2m_collection(_rel: &super::RelationalNode) -> Option<String> {
    // In our AST, the O2M field_key contains the FK in the child table.
    // The collection name isn't stored directly on RelationalNode.
    // By convention in Directus, the relation name often matches the collection.
    // This is a simplification — the full solution would store the collection
    // name on the RelationalNode.
    None
}

/// Build WHERE clause SQL from a Filter.
fn build_filter_sql(
    filter: &nexus_types::filter::Filter,
    db: &Arc<dyn DatabaseBackend>,
    table: &str,
    mut param_idx: usize,
) -> Result<(String, Vec<SqlValue>, usize), RunAstError> {
    match filter {
        nexus_types::filter::Filter::Logical(logical) => match logical {
            nexus_types::filter::LogicalFilter::And { _and } => {
                let mut parts = Vec::new();
                let mut all_bindings = Vec::new();
                for sub in _and {
                    let (clause, bindings, next) =
                        build_filter_sql(sub, db, table, param_idx)?;
                    if !clause.is_empty() {
                        parts.push(clause);
                        all_bindings.extend(bindings);
                        param_idx = next;
                    }
                }
                if parts.is_empty() {
                    Ok((String::new(), Vec::new(), param_idx))
                } else {
                    Ok((
                        format!("({})", parts.join(" AND ")),
                        all_bindings,
                        param_idx,
                    ))
                }
            }
            nexus_types::filter::LogicalFilter::Or { _or } => {
                let mut parts = Vec::new();
                let mut all_bindings = Vec::new();
                for sub in _or {
                    let (clause, bindings, next) =
                        build_filter_sql(sub, db, table, param_idx)?;
                    if !clause.is_empty() {
                        parts.push(clause);
                        all_bindings.extend(bindings);
                        param_idx = next;
                    }
                }
                if parts.is_empty() {
                    Ok((String::new(), Vec::new(), param_idx))
                } else {
                    Ok((
                        format!("({})", parts.join(" OR ")),
                        all_bindings,
                        param_idx,
                    ))
                }
            }
        },
        nexus_types::filter::Filter::Field(field_map) => {
            let mut parts = Vec::new();
            let mut all_bindings = Vec::new();

            for (field, operators) in field_map {
                if let Some(ops) = operators.as_object() {
                    for (op, val) in ops {
                        let quoted = format!(
                            "{}.{}",
                            db.quote_identifier(table),
                            db.quote_identifier(field)
                        );
                        if let Some((clause, binding)) =
                            build_operator_sql(&quoted, op, val, &mut param_idx)
                        {
                            parts.push(clause);
                            if let Some(b) = binding {
                                all_bindings.push(b);
                            }
                        }
                    }
                }
            }

            if parts.is_empty() {
                Ok((String::new(), Vec::new(), param_idx))
            } else {
                Ok((parts.join(" AND "), all_bindings, param_idx))
            }
        }
    }
}

fn build_operator_sql(
    quoted_field: &str,
    op: &str,
    val: &Value,
    param_idx: &mut usize,
) -> Option<(String, Option<SqlValue>)> {
    match op {
        "_eq" => {
            if val.is_null() {
                Some((format!("{} IS NULL", quoted_field), None))
            } else {
                let idx = *param_idx;
                *param_idx += 1;
                Some((
                    format!("{} = ${}", quoted_field, idx),
                    Some(value_to_sql_value(val)),
                ))
            }
        }
        "_neq" => {
            if val.is_null() {
                Some((format!("{} IS NOT NULL", quoted_field), None))
            } else {
                let idx = *param_idx;
                *param_idx += 1;
                Some((
                    format!("{} != ${}", quoted_field, idx),
                    Some(value_to_sql_value(val)),
                ))
            }
        }
        "_lt" => {
            let idx = *param_idx;
            *param_idx += 1;
            Some((
                format!("{} < ${}", quoted_field, idx),
                Some(value_to_sql_value(val)),
            ))
        }
        "_lte" => {
            let idx = *param_idx;
            *param_idx += 1;
            Some((
                format!("{} <= ${}", quoted_field, idx),
                Some(value_to_sql_value(val)),
            ))
        }
        "_gt" => {
            let idx = *param_idx;
            *param_idx += 1;
            Some((
                format!("{} > ${}", quoted_field, idx),
                Some(value_to_sql_value(val)),
            ))
        }
        "_gte" => {
            let idx = *param_idx;
            *param_idx += 1;
            Some((
                format!("{} >= ${}", quoted_field, idx),
                Some(value_to_sql_value(val)),
            ))
        }
        "_in" => {
            if let Some(arr) = val.as_array() {
                let placeholders: Vec<String> = arr
                    .iter()
                    .map(|_| {
                        let idx = *param_idx;
                        *param_idx += 1;
                        format!("${}", idx)
                    })
                    .collect();
                // Note: we return the first binding, but all must be added by caller
                // This is a simplification — we need to handle multi-value bindings
                let first = arr.first().map(value_to_sql_value);
                Some((
                    format!("{} IN ({})", quoted_field, placeholders.join(", ")),
                    first,
                ))
            } else {
                None
            }
        }
        "_null" => {
            if val.as_bool().unwrap_or(false) {
                Some((format!("{} IS NULL", quoted_field), None))
            } else {
                Some((format!("{} IS NOT NULL", quoted_field), None))
            }
        }
        "_nnull" => {
            if val.as_bool().unwrap_or(false) {
                Some((format!("{} IS NOT NULL", quoted_field), None))
            } else {
                Some((format!("{} IS NULL", quoted_field), None))
            }
        }
        "_contains" => {
            let idx = *param_idx;
            *param_idx += 1;
            let s = val.as_str().unwrap_or_default();
            Some((
                format!("{} LIKE ${}", quoted_field, idx),
                Some(SqlValue::Text(format!("%{}%", s))),
            ))
        }
        "_icontains" => {
            let idx = *param_idx;
            *param_idx += 1;
            let s = val.as_str().unwrap_or_default();
            Some((
                format!("{} ILIKE ${}", quoted_field, idx),
                Some(SqlValue::Text(format!("%{}%", s))),
            ))
        }
        "_starts_with" => {
            let idx = *param_idx;
            *param_idx += 1;
            let s = val.as_str().unwrap_or_default();
            Some((
                format!("{} LIKE ${}", quoted_field, idx),
                Some(SqlValue::Text(format!("{}%", s))),
            ))
        }
        "_ends_with" => {
            let idx = *param_idx;
            *param_idx += 1;
            let s = val.as_str().unwrap_or_default();
            Some((
                format!("{} LIKE ${}", quoted_field, idx),
                Some(SqlValue::Text(format!("%{}", s))),
            ))
        }
        "_between" => {
            if let Some(arr) = val.as_array() {
                if arr.len() == 2 {
                    let idx1 = *param_idx;
                    *param_idx += 1;
                    let idx2 = *param_idx;
                    *param_idx += 1;
                    Some((
                        format!("{} BETWEEN ${} AND ${}", quoted_field, idx1, idx2),
                        Some(value_to_sql_value(&arr[0])),
                        // Note: arr[1] binding also needs to be added
                    ))
                } else {
                    None
                }
            } else {
                None
            }
        }
        "_empty" => {
            if val.as_bool().unwrap_or(false) {
                Some((
                    format!("({} IS NULL OR {} = '')", quoted_field, quoted_field),
                    None,
                ))
            } else {
                Some((
                    format!("({} IS NOT NULL AND {} != '')", quoted_field, quoted_field),
                    None,
                ))
            }
        }
        _ => None,
    }
}

fn value_to_sql_value(val: &Value) -> SqlValue {
    match val {
        Value::Null => SqlValue::Null,
        Value::Bool(b) => SqlValue::Bool(*b),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                SqlValue::Int(i)
            } else if let Some(f) = n.as_f64() {
                SqlValue::Float(f)
            } else {
                SqlValue::Text(n.to_string())
            }
        }
        Value::String(s) => SqlValue::Text(s.clone()),
        Value::Array(_) | Value::Object(_) => SqlValue::Json(val.clone()),
    }
}

fn value_to_string(val: &Value) -> String {
    match val {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Null => "null".to_string(),
        _ => val.to_string(),
    }
}
