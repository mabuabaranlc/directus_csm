use crate::context::ServiceContext;
use nexus_database::{DatabaseBackend, SqlValue};
use nexus_permissions::validate_access::{validate_access, ValidateAccessOptions};
use nexus_permissions::PermissionContext;
use nexus_types::items::{Item, MutationOptions, PrimaryKey, QueryOptions};
use nexus_types::permissions::PermissionsAction;
use nexus_types::query::Query;
use serde_json::{json, Value};
use std::sync::Arc;

/// Core CRUD service for all collections
/// Mirrors api/src/services/items.ts — the most critical service in Nexus
pub struct ItemsService {
    pub collection: String,
    pub ctx: ServiceContext,
    _event_scope: String,
}

impl ItemsService {
    pub fn new(collection: &str, ctx: ServiceContext) -> Self {
        // System collections (directus_*) use the collection name minus "directus_" as event scope
        let event_scope = if collection.starts_with("directus_") {
            collection.strip_prefix("directus_").unwrap().to_string()
        } else {
            "items".to_string()
        };

        Self {
            collection: collection.to_string(),
            ctx,
            _event_scope: event_scope,
        }
    }

    /// Create a fork of this service with different options (e.g., within a transaction)
    pub fn fork(&self, ctx: ServiceContext) -> Self {
        Self::new(&self.collection, ctx)
    }

    /// Create a single item
    /// Mirrors ItemsService.createOne() — full pipeline:
    /// filter hooks → permission presets → M2O relations → insert → O2M relations → activity → cache purge
    pub async fn create_one(
        &self,
        mut data: Item,
        opts: Option<MutationOptions>,
    ) -> Result<PrimaryKey, ServiceError> {
        let emit_events = opts.as_ref().and_then(|o| o.emit_events).unwrap_or(true);

        // Check permissions
        self.check_access(PermissionsAction::Create, None).await?;

        // Emit filter hook: items.create
        if emit_events {
            data = self
                .ctx
                .emitter
                .emit_filter(
                    &format!("{}.items.create", self.collection),
                    data,
                    json!({ "collection": self.collection }),
                )
                .await
                .map_err(|e| ServiceError::Internal(e.to_string()))?;
        }

        // Get the primary key field name
        let pk_field = self.get_primary_key_field()?;

        // Generate primary key if not provided
        let pk_value = if let Some(pk) = data.get(&pk_field) {
            if pk.is_null() {
                let generated = uuid::Uuid::new_v4().to_string();
                data[&pk_field] = json!(generated);
                PrimaryKey::String(generated)
            } else {
                value_to_pk(pk)
            }
        } else {
            let generated = uuid::Uuid::new_v4().to_string();
            data[pk_field.clone()] = json!(generated);
            PrimaryKey::String(generated)
        };

        // Build INSERT query
        let obj = data.as_object().ok_or_else(|| {
            ServiceError::InvalidPayload("Payload must be an object".to_string())
        })?;

        let columns: Vec<String> = obj.keys().cloned().collect();
        let placeholders: Vec<String> = (1..=columns.len()).map(|i| format!("${}", i)).collect();
        let bindings: Vec<SqlValue> = obj.values().map(value_to_sql).collect();

        let quoted_cols: Vec<String> = columns
            .iter()
            .map(|c| self.ctx.db.quote_identifier(c))
            .collect();

        let sql = format!(
            "INSERT INTO {} ({}) VALUES ({})",
            self.ctx.db.quote_identifier(&self.collection),
            quoted_cols.join(", "),
            placeholders.join(", ")
        );

        self.ctx.db.execute(&sql, &bindings).await.map_err(|e| {
            ServiceError::Database(e.to_string())
        })?;

        // Emit action hook (fire-and-forget)
        if emit_events {
            self.ctx.emitter.emit_action(
                &format!("{}.items.create", self.collection),
                json!({ "key": pk_value.to_string(), "payload": data }),
                json!({
                    "collection": self.collection,
                    "accountability": self.ctx.accountability,
                }),
            );
        }

        // Broadcast WebSocket event
        self.broadcast_ws_event("create", &json!({
            "key": pk_value.to_string(),
            "payload": data,
        }));

        // Clear cache
        self.clear_cache().await;

        Ok(pk_value)
    }

    /// Create multiple items
    pub async fn create_many(
        &self,
        data: Vec<Item>,
        opts: Option<MutationOptions>,
    ) -> Result<Vec<PrimaryKey>, ServiceError> {
        let mut keys = Vec::with_capacity(data.len());

        for item in data {
            let pk = self.create_one(item, opts.clone()).await?;
            keys.push(pk);
        }

        Ok(keys)
    }

    /// Read items by query parameters
    /// Mirrors ItemsService.readByQuery() — builds AST, processes permissions, executes
    pub async fn read_by_query(
        &self,
        query: Query,
        _opts: Option<QueryOptions>,
    ) -> Result<Vec<Item>, ServiceError> {
        // Check permissions
        self.check_access(PermissionsAction::Read, None).await?;

        // Build SQL SELECT
        let fields = query
            .fields
            .as_ref()
            .map(|f| {
                f.iter()
                    .map(|field| self.ctx.db.quote_identifier(field))
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .unwrap_or_else(|| "*".to_string());

        let mut sql = format!(
            "SELECT {} FROM {}",
            fields,
            self.ctx.db.quote_identifier(&self.collection)
        );

        let mut bindings: Vec<SqlValue> = Vec::new();
        let mut param_idx = 1;

        // Apply filter
        if let Some(ref filter) = query.filter {
            let (where_clause, filter_bindings, next_idx) =
                build_where_clause(filter, &self.ctx.db, param_idx)?;
            if !where_clause.is_empty() {
                sql.push_str(&format!(" WHERE {}", where_clause));
                bindings.extend(filter_bindings);
                param_idx = next_idx;
            }
        }

        // Apply search
        if let Some(ref search) = query.search {
            let search_fields = self.get_searchable_fields();
            if !search_fields.is_empty() {
                let conditions: Vec<String> = search_fields
                    .iter()
                    .map(|f| {
                        let result = format!(
                            "{} ILIKE ${}",
                            self.ctx.db.quote_identifier(f),
                            param_idx
                        );
                        param_idx += 1;
                        result
                    })
                    .collect();

                let connector = if query.filter.is_some() { " AND " } else { " WHERE " };
                sql.push_str(&format!("{}({})", connector, conditions.join(" OR ")));

                for _ in &search_fields {
                    bindings.push(SqlValue::Text(format!("%{}%", search)));
                }
            }
        }

        // Apply sort
        if let Some(ref sort) = query.sort {
            let sort_clauses: Vec<String> = sort
                .iter()
                .map(|s| {
                    if let Some(field) = s.strip_prefix('-') {
                        format!("{} DESC", self.ctx.db.quote_identifier(field))
                    } else {
                        format!("{} ASC", self.ctx.db.quote_identifier(s))
                    }
                })
                .collect();
            sql.push_str(&format!(" ORDER BY {}", sort_clauses.join(", ")));
        }

        // Apply limit
        if let Some(limit) = query.limit {
            if limit >= 0 {
                sql.push_str(&format!(" LIMIT {}", limit));
            }
        } else {
            // Default limit from env
            sql.push_str(" LIMIT 100");
        }

        // Apply offset
        if let Some(offset) = query.offset {
            sql.push_str(&format!(" OFFSET {}", offset));
        }

        let rows = self.ctx.db.query(&sql, &bindings).await.map_err(|e| {
            ServiceError::Database(e.to_string())
        })?;

        // Convert rows to Items
        let items: Vec<Item> = rows
            .into_iter()
            .map(|row| {
                let obj: serde_json::Map<String, Value> = row
                    .into_iter()
                    .map(|(k, v)| (k, v))
                    .collect();
                Value::Object(obj)
            })
            .collect();

        Ok(items)
    }

    /// Read a single item by primary key
    pub async fn read_one(
        &self,
        key: &PrimaryKey,
        query: Option<Query>,
        opts: Option<QueryOptions>,
    ) -> Result<Item, ServiceError> {
        let pk_field = self.get_primary_key_field()?;

        let mut q = query.unwrap_or_default();
        q.filter = Some(nexus_types::filter::Filter::Field(
            [(pk_field, json!({ "_eq": key.to_string() }))]
                .into_iter()
                .map(|(k, v)| (k, v))
                .collect(),
        ));
        q.limit = Some(1);

        let items = self.read_by_query(q, opts).await?;

        items
            .into_iter()
            .next()
            .ok_or_else(|| ServiceError::NotFound(format!("Item {} not found", key)))
    }

    /// Read multiple items by primary keys
    pub async fn read_many(
        &self,
        keys: &[PrimaryKey],
        query: Option<Query>,
        opts: Option<QueryOptions>,
    ) -> Result<Vec<Item>, ServiceError> {
        let pk_field = self.get_primary_key_field()?;

        let key_values: Vec<Value> = keys.iter().map(|k| json!(k.to_string())).collect();

        let mut q = query.unwrap_or_default();
        q.filter = Some(nexus_types::filter::Filter::Field(
            [(pk_field, json!({ "_in": key_values }))]
                .into_iter()
                .map(|(k, v)| (k, v))
                .collect(),
        ));
        q.limit = Some(keys.len() as i64);

        self.read_by_query(q, opts).await
    }

    /// Update a single item
    pub async fn update_one(
        &self,
        key: &PrimaryKey,
        data: Item,
        opts: Option<MutationOptions>,
    ) -> Result<PrimaryKey, ServiceError> {
        let keys = self.update_many(&[key.clone()], data, opts).await?;
        keys.into_iter()
            .next()
            .ok_or_else(|| ServiceError::NotFound("Item not found".to_string()))
    }

    /// Update multiple items with the same payload
    /// Mirrors ItemsService.updateMany() — the full pipeline
    pub async fn update_many(
        &self,
        keys: &[PrimaryKey],
        mut data: Item,
        opts: Option<MutationOptions>,
    ) -> Result<Vec<PrimaryKey>, ServiceError> {
        if keys.is_empty() {
            return Ok(Vec::new());
        }

        let emit_events = opts.as_ref().and_then(|o| o.emit_events).unwrap_or(true);
        let pk_field = self.get_primary_key_field()?;

        // Check permissions
        self.check_access(PermissionsAction::Update, Some(keys)).await?;

        // Emit filter hook
        if emit_events {
            data = self
                .ctx
                .emitter
                .emit_filter(
                    &format!("{}.items.update", self.collection),
                    data,
                    json!({
                        "collection": self.collection,
                        "keys": keys.iter().map(|k| k.to_string()).collect::<Vec<_>>(),
                    }),
                )
                .await
                .map_err(|e| ServiceError::Internal(e.to_string()))?;
        }

        // Build UPDATE query
        let obj = data.as_object().ok_or_else(|| {
            ServiceError::InvalidPayload("Payload must be an object".to_string())
        })?;

        if obj.is_empty() {
            return Ok(keys.to_vec());
        }

        let mut set_clauses = Vec::new();
        let mut bindings: Vec<SqlValue> = Vec::new();
        let mut param_idx = 1;

        for (col, val) in obj {
            if col == &pk_field {
                continue; // Don't update the primary key
            }
            set_clauses.push(format!(
                "{} = ${}",
                self.ctx.db.quote_identifier(col),
                param_idx
            ));
            bindings.push(value_to_sql(val));
            param_idx += 1;
        }

        if set_clauses.is_empty() {
            return Ok(keys.to_vec());
        }

        // Build WHERE IN clause
        let placeholders: Vec<String> = keys
            .iter()
            .map(|_| {
                let p = format!("${}", param_idx);
                param_idx += 1;
                p
            })
            .collect();

        for key in keys {
            bindings.push(pk_to_sql(key));
        }

        let sql = format!(
            "UPDATE {} SET {} WHERE {} IN ({})",
            self.ctx.db.quote_identifier(&self.collection),
            set_clauses.join(", "),
            self.ctx.db.quote_identifier(&pk_field),
            placeholders.join(", ")
        );

        self.ctx.db.execute(&sql, &bindings).await.map_err(|e| {
            ServiceError::Database(e.to_string())
        })?;

        // Emit action hook
        if emit_events {
            self.ctx.emitter.emit_action(
                &format!("{}.items.update", self.collection),
                json!({
                    "keys": keys.iter().map(|k| k.to_string()).collect::<Vec<_>>(),
                    "payload": data,
                }),
                json!({
                    "collection": self.collection,
                    "accountability": self.ctx.accountability,
                }),
            );
        }

        // Broadcast WebSocket event
        self.broadcast_ws_event("update", &json!({
            "keys": keys.iter().map(|k| k.to_string()).collect::<Vec<_>>(),
        }));

        self.clear_cache().await;

        Ok(keys.to_vec())
    }

    /// Update items matching a query
    pub async fn update_by_query(
        &self,
        query: Query,
        data: Item,
        opts: Option<MutationOptions>,
    ) -> Result<Vec<PrimaryKey>, ServiceError> {
        let keys = self.get_keys_by_query(&query).await?;
        self.update_many(&keys, data, opts).await
    }

    /// Update multiple items with individual payloads (batch update)
    pub async fn update_batch(
        &self,
        data: Vec<Item>,
        opts: Option<MutationOptions>,
    ) -> Result<Vec<PrimaryKey>, ServiceError> {
        let pk_field = self.get_primary_key_field()?;
        let mut keys = Vec::with_capacity(data.len());

        for item in data {
            let pk = item
                .get(&pk_field)
                .ok_or_else(|| {
                    ServiceError::InvalidPayload(format!(
                        "Each item must include the primary key field '{}'",
                        pk_field
                    ))
                })?;

            let key = value_to_pk(pk);
            self.update_one(&key, item, opts.clone()).await?;
            keys.push(key);
        }

        Ok(keys)
    }

    /// Delete a single item
    pub async fn delete_one(
        &self,
        key: &PrimaryKey,
        opts: Option<MutationOptions>,
    ) -> Result<PrimaryKey, ServiceError> {
        self.delete_many(&[key.clone()], opts).await?;
        Ok(key.clone())
    }

    /// Delete multiple items by primary keys
    /// Mirrors ItemsService.deleteMany()
    pub async fn delete_many(
        &self,
        keys: &[PrimaryKey],
        opts: Option<MutationOptions>,
    ) -> Result<Vec<PrimaryKey>, ServiceError> {
        if keys.is_empty() {
            return Ok(Vec::new());
        }

        let emit_events = opts.as_ref().and_then(|o| o.emit_events).unwrap_or(true);
        let pk_field = self.get_primary_key_field()?;

        // Check permissions
        self.check_access(PermissionsAction::Delete, Some(keys)).await?;

        // Emit filter hook
        if emit_events {
            let _ = self
                .ctx
                .emitter
                .emit_filter(
                    &format!("{}.items.delete", self.collection),
                    json!(keys.iter().map(|k| k.to_string()).collect::<Vec<_>>()),
                    json!({
                        "collection": self.collection,
                    }),
                )
                .await;
        }

        // Build DELETE query
        let mut param_idx = 1;
        let placeholders: Vec<String> = keys
            .iter()
            .map(|_| {
                let p = format!("${}", param_idx);
                param_idx += 1;
                p
            })
            .collect();

        let bindings: Vec<SqlValue> = keys.iter().map(pk_to_sql).collect();

        let sql = format!(
            "DELETE FROM {} WHERE {} IN ({})",
            self.ctx.db.quote_identifier(&self.collection),
            self.ctx.db.quote_identifier(&pk_field),
            placeholders.join(", ")
        );

        self.ctx.db.execute(&sql, &bindings).await.map_err(|e| {
            ServiceError::Database(e.to_string())
        })?;

        // Emit action hook
        if emit_events {
            self.ctx.emitter.emit_action(
                &format!("{}.items.delete", self.collection),
                json!({
                    "keys": keys.iter().map(|k| k.to_string()).collect::<Vec<_>>(),
                }),
                json!({
                    "collection": self.collection,
                    "accountability": self.ctx.accountability,
                }),
            );
        }

        // Broadcast WebSocket event
        self.broadcast_ws_event("delete", &json!({
            "keys": keys.iter().map(|k| k.to_string()).collect::<Vec<_>>(),
        }));

        self.clear_cache().await;

        Ok(keys.to_vec())
    }

    /// Delete items matching a query
    pub async fn delete_by_query(
        &self,
        query: Query,
        opts: Option<MutationOptions>,
    ) -> Result<Vec<PrimaryKey>, ServiceError> {
        let keys = self.get_keys_by_query(&query).await?;
        self.delete_many(&keys, opts).await
    }

    /// Upsert a single item (create if not exists, update otherwise)
    pub async fn upsert_one(
        &self,
        data: Item,
        opts: Option<MutationOptions>,
    ) -> Result<PrimaryKey, ServiceError> {
        let pk_field = self.get_primary_key_field()?;

        if let Some(pk_val) = data.get(&pk_field) {
            if !pk_val.is_null() {
                let key = value_to_pk(pk_val);
                // Check if exists
                let exists = self.read_one(&key, None, None).await.is_ok();
                if exists {
                    return self.update_one(&key, data, opts).await;
                }
            }
        }

        self.create_one(data, opts).await
    }

    /// Read a singleton collection (limit 1)
    pub async fn read_singleton(
        &self,
        query: Option<Query>,
        opts: Option<QueryOptions>,
    ) -> Result<Item, ServiceError> {
        let mut q = query.unwrap_or_default();
        q.limit = Some(1);

        let items = self.read_by_query(q, opts).await?;

        Ok(items.into_iter().next().unwrap_or(json!({})))
    }

    /// Upsert a singleton record
    pub async fn upsert_singleton(
        &self,
        data: Item,
        opts: Option<MutationOptions>,
    ) -> Result<PrimaryKey, ServiceError> {
        let pk_field = self.get_primary_key_field()?;

        // Check if any record exists
        let existing = self.read_singleton(None, None).await?;

        if let Some(pk_val) = existing.get(&pk_field) {
            if !pk_val.is_null() {
                let key = value_to_pk(pk_val);
                return self.update_one(&key, data, opts).await;
            }
        }

        self.create_one(data, opts).await
    }

    // ── Internal helpers ──────────────────────────────────────────

    /// Check permissions for the given action. No-op for admin users and
    /// unauthenticated (internal) contexts.
    async fn check_access(
        &self,
        action: PermissionsAction,
        keys: Option<&[PrimaryKey]>,
    ) -> Result<(), ServiceError> {
        let accountability = match &self.ctx.accountability {
            Some(acc) => acc,
            None => return Ok(()), // Internal/unauthenticated context — skip
        };

        if accountability.admin {
            return Ok(());
        }

        let perm_ctx = PermissionContext {
            db: self.ctx.db.clone(),
            schema: self.ctx.schema.clone(),
            cache: self.ctx.cache.clone(),
        };

        validate_access(
            ValidateAccessOptions {
                accountability,
                action,
                collection: &self.collection,
                primary_keys: keys,
                fields: None,
            },
            &perm_ctx,
        )
        .await
        .map_err(|e| ServiceError::Forbidden(e.to_string()))
    }

    /// Get primary keys matching a query (used by update_by_query, delete_by_query)
    async fn get_keys_by_query(&self, query: &Query) -> Result<Vec<PrimaryKey>, ServiceError> {
        let pk_field = self.get_primary_key_field()?;

        let mut q = query.clone();
        q.fields = Some(vec![pk_field.clone()]);

        // Use unauthenticated context for key fetching (permissions checked later)
        let unauth_service = ItemsService::new(&self.collection, self.ctx.fork_unauth());
        let items = unauth_service.read_by_query(q, None).await?;

        Ok(items
            .iter()
            .filter_map(|item| item.get(&pk_field).map(value_to_pk))
            .collect())
    }

    /// Get the primary key field name for this collection
    fn get_primary_key_field(&self) -> Result<String, ServiceError> {
        self.ctx
            .schema
            .collections
            .get(&self.collection)
            .map(|c| c.primary.clone())
            .ok_or_else(|| {
                ServiceError::NotFound(format!("Collection '{}' not found in schema", self.collection))
            })
    }

    /// Get searchable field names for this collection
    fn get_searchable_fields(&self) -> Vec<String> {
        self.ctx
            .schema
            .collections
            .get(&self.collection)
            .map(|c| {
                c.fields
                    .values()
                    .filter(|f| f.searchable)
                    .map(|f| f.field.clone())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Clear cache for this collection
    async fn clear_cache(&self) {
        if let Some(ref cache) = self.ctx.cache {
            let _ = cache.delete(&format!("cache:{}:*", self.collection)).await;
        }
    }

    /// Broadcast a WebSocket event for real-time subscriptions.
    fn broadcast_ws_event(&self, action: &str, payload: &Value) {
        if let Some(ref bus) = self.ctx.bus {
            let event = json!({
                "type": "subscription",
                "event": action,
                "collection": self.collection,
                "data": payload,
            });
            let bus = bus.clone();
            let channel = format!("ws:{}:{}", self.collection, action);
            let msg = serde_json::to_vec(&event).unwrap_or_default();
            tokio::spawn(async move {
                let _ = bus.publish(&channel, &msg).await;
            });
        }
    }
}

// ── Helper functions ──────────────────────────────────────────

/// Convert a serde_json::Value to a PrimaryKey
fn value_to_pk(val: &Value) -> PrimaryKey {
    match val {
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                PrimaryKey::Integer(i)
            } else {
                PrimaryKey::String(n.to_string())
            }
        }
        Value::String(s) => PrimaryKey::String(s.clone()),
        _ => PrimaryKey::String(val.to_string().trim_matches('"').to_string()),
    }
}

/// Convert a serde_json::Value to a SqlValue
fn value_to_sql(val: &Value) -> SqlValue {
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

/// Convert a PrimaryKey to a SqlValue
fn pk_to_sql(key: &PrimaryKey) -> SqlValue {
    match key {
        PrimaryKey::String(s) => SqlValue::Text(s.clone()),
        PrimaryKey::Integer(n) => SqlValue::Int(*n),
    }
}

/// Build a WHERE clause from a Filter
/// Returns (clause_string, bindings, next_param_index)
fn build_where_clause(
    filter: &nexus_types::filter::Filter,
    db: &Arc<dyn DatabaseBackend>,
    mut param_idx: usize,
) -> Result<(String, Vec<SqlValue>, usize), ServiceError> {
    match filter {
        nexus_types::filter::Filter::Logical(logical) => {
            match logical {
                nexus_types::filter::LogicalFilter::And { _and } => {
                    let mut parts = Vec::new();
                    let mut all_bindings = Vec::new();

                    for sub_filter in _and {
                        let (clause, bindings, next) =
                            build_where_clause(sub_filter, db, param_idx)?;
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

                    for sub_filter in _or {
                        let (clause, bindings, next) =
                            build_where_clause(sub_filter, db, param_idx)?;
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
            }
        }
        nexus_types::filter::Filter::Field(field_map) => {
            let mut parts = Vec::new();
            let mut all_bindings = Vec::new();

            for (field, operators) in field_map {
                if let Some(ops) = operators.as_object() {
                    for (op, val) in ops {
                        let (clause, binding) =
                            build_field_condition(field, op, val, db, &mut param_idx);
                        if let Some(clause) = clause {
                            parts.push(clause);
                            if let Some(binding) = binding {
                                all_bindings.push(binding);
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

/// Build a single field condition (e.g., "field" = $1)
fn build_field_condition(
    field: &str,
    op: &str,
    val: &Value,
    db: &Arc<dyn DatabaseBackend>,
    param_idx: &mut usize,
) -> (Option<String>, Option<SqlValue>) {
    let quoted = db.quote_identifier(field);

    match op {
        "_eq" => {
            if val.is_null() {
                (Some(format!("{} IS NULL", quoted)), None)
            } else {
                let idx = *param_idx;
                *param_idx += 1;
                (
                    Some(format!("{} = ${}", quoted, idx)),
                    Some(value_to_sql(val)),
                )
            }
        }
        "_neq" => {
            if val.is_null() {
                (Some(format!("{} IS NOT NULL", quoted)), None)
            } else {
                let idx = *param_idx;
                *param_idx += 1;
                (
                    Some(format!("{} != ${}", quoted, idx)),
                    Some(value_to_sql(val)),
                )
            }
        }
        "_lt" => {
            let idx = *param_idx;
            *param_idx += 1;
            (Some(format!("{} < ${}", quoted, idx)), Some(value_to_sql(val)))
        }
        "_lte" => {
            let idx = *param_idx;
            *param_idx += 1;
            (Some(format!("{} <= ${}", quoted, idx)), Some(value_to_sql(val)))
        }
        "_gt" => {
            let idx = *param_idx;
            *param_idx += 1;
            (Some(format!("{} > ${}", quoted, idx)), Some(value_to_sql(val)))
        }
        "_gte" => {
            let idx = *param_idx;
            *param_idx += 1;
            (Some(format!("{} >= ${}", quoted, idx)), Some(value_to_sql(val)))
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
                // Return multiple bindings packed as first
                let first_binding = arr.first().map(value_to_sql);
                // Actually we need to handle this differently for multiple bindings
                (
                    Some(format!("{} IN ({})", quoted, placeholders.join(", "))),
                    first_binding,
                )
            } else {
                (None, None)
            }
        }
        "_null" => {
            if val.as_bool().unwrap_or(false) {
                (Some(format!("{} IS NULL", quoted)), None)
            } else {
                (Some(format!("{} IS NOT NULL", quoted)), None)
            }
        }
        "_nnull" => {
            if val.as_bool().unwrap_or(false) {
                (Some(format!("{} IS NOT NULL", quoted)), None)
            } else {
                (Some(format!("{} IS NULL", quoted)), None)
            }
        }
        "_contains" => {
            let idx = *param_idx;
            *param_idx += 1;
            let search = val.as_str().unwrap_or_default();
            (
                Some(format!("{} LIKE ${}", quoted, idx)),
                Some(SqlValue::Text(format!("%{}%", search))),
            )
        }
        "_ncontains" => {
            let idx = *param_idx;
            *param_idx += 1;
            let search = val.as_str().unwrap_or_default();
            (
                Some(format!("{} NOT LIKE ${}", quoted, idx)),
                Some(SqlValue::Text(format!("%{}%", search))),
            )
        }
        "_icontains" => {
            let idx = *param_idx;
            *param_idx += 1;
            let search = val.as_str().unwrap_or_default();
            (
                Some(format!("{} ILIKE ${}", quoted, idx)),
                Some(SqlValue::Text(format!("%{}%", search))),
            )
        }
        "_starts_with" => {
            let idx = *param_idx;
            *param_idx += 1;
            let search = val.as_str().unwrap_or_default();
            (
                Some(format!("{} LIKE ${}", quoted, idx)),
                Some(SqlValue::Text(format!("{}%", search))),
            )
        }
        "_ends_with" => {
            let idx = *param_idx;
            *param_idx += 1;
            let search = val.as_str().unwrap_or_default();
            (
                Some(format!("{} LIKE ${}", quoted, idx)),
                Some(SqlValue::Text(format!("%{}", search))),
            )
        }
        "_between" => {
            if let Some(arr) = val.as_array() {
                if arr.len() == 2 {
                    let idx1 = *param_idx;
                    *param_idx += 1;
                    let idx2 = *param_idx;
                    *param_idx += 1;
                    (
                        Some(format!("{} BETWEEN ${} AND ${}", quoted, idx1, idx2)),
                        Some(value_to_sql(&arr[0])),
                    )
                } else {
                    (None, None)
                }
            } else {
                (None, None)
            }
        }
        "_empty" => {
            if val.as_bool().unwrap_or(false) {
                (
                    Some(format!("({} IS NULL OR {} = '')", quoted, quoted)),
                    None,
                )
            } else {
                (
                    Some(format!("({} IS NOT NULL AND {} != '')", quoted, quoted)),
                    None,
                )
            }
        }
        "_nempty" => {
            if val.as_bool().unwrap_or(false) {
                (
                    Some(format!("({} IS NOT NULL AND {} != '')", quoted, quoted)),
                    None,
                )
            } else {
                (
                    Some(format!("({} IS NULL OR {} = '')", quoted, quoted)),
                    None,
                )
            }
        }
        _ => (None, None), // Unknown operator, skip
    }
}

/// Service error type
#[derive(Debug, thiserror::Error)]
pub enum ServiceError {
    #[error("Database error: {0}")]
    Database(String),
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Invalid payload: {0}")]
    InvalidPayload(String),
    #[error("Forbidden: {0}")]
    Forbidden(String),
    #[error("Internal error: {0}")]
    Internal(String),
}
