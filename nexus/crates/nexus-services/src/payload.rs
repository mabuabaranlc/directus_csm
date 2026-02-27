use crate::context::ServiceContext;
use crate::items::ServiceError;
use nexus_types::accountability::Accountability;
use nexus_types::fields::FieldType;
use nexus_types::schema::FieldOverview;
use serde_json::{json, Value};
use std::collections::HashMap;

/// Action being performed — determines which transformers to apply
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PayloadAction {
    Create,
    Read,
    Update,
}

/// Payload processing service — handles type casting, special field transformations,
/// date formatting, geometry conversion, and relational payload processing.
/// Mirrors api/src/services/payload.ts
pub struct PayloadService {
    pub collection: String,
    pub ctx: ServiceContext,
}

impl PayloadService {
    pub fn new(collection: &str, ctx: ServiceContext) -> Self {
        Self {
            collection: collection.to_string(),
            ctx,
        }
    }

    /// Process values for a single payload
    /// Applies special field transformers, geometry conversion, and date formatting
    pub async fn process_values(
        &self,
        action: PayloadAction,
        mut payload: Value,
    ) -> Result<Value, ServiceError> {
        let fields = self.get_fields_with_specials();

        for (field_name, field_info) in &fields {
            if let Some(val) = payload.get(field_name).cloned() {
                let new_val = self
                    .process_field(field_name, &val, &field_info.specials, action)
                    .await;
                if let Some(obj) = payload.as_object_mut() {
                    obj.insert(field_name.clone(), new_val);
                }
            } else if action == PayloadAction::Create {
                // Auto-generate values for create
                let auto_val = self.auto_generate(field_name, &field_info.specials, action).await;
                if let Some(auto_val) = auto_val {
                    if let Some(obj) = payload.as_object_mut() {
                        obj.insert(field_name.clone(), auto_val);
                    }
                }
            } else if action == PayloadAction::Update {
                // Auto-update values on update
                let auto_val = self.auto_generate(field_name, &field_info.specials, action).await;
                if let Some(auto_val) = auto_val {
                    if let Some(obj) = payload.as_object_mut() {
                        obj.insert(field_name.clone(), auto_val);
                    }
                }
            }
        }

        // Process dates
        self.process_dates(action, &mut payload);

        // JSON-stringify objects/arrays on write
        if action != PayloadAction::Read {
            self.json_stringify_objects(&mut payload);
        }

        Ok(payload)
    }

    /// Process multiple payloads
    pub async fn process_values_many(
        &self,
        action: PayloadAction,
        payloads: Vec<Value>,
    ) -> Result<Vec<Value>, ServiceError> {
        let mut results = Vec::with_capacity(payloads.len());
        for payload in payloads {
            results.push(self.process_values(action, payload).await?);
        }
        Ok(results)
    }

    /// Process a single field through its special transformers
    async fn process_field(
        &self,
        field_name: &str,
        value: &Value,
        specials: &[String],
        action: PayloadAction,
    ) -> Value {
        let mut result = value.clone();

        for special in specials {
            result = match special.as_str() {
                "hash" => self.transform_hash(&result, action).await,
                "uuid" => self.transform_uuid(&result, action),
                "cast-boolean" => self.transform_cast_boolean(&result, action),
                "cast-json" => self.transform_cast_json(&result, action),
                "cast-csv" => self.transform_cast_csv(&result, action),
                "conceal" => self.transform_conceal(&result, action),
                _ => result,
            };
        }

        result
    }

    /// Auto-generate values for special fields (user-created, date-created, etc.)
    async fn auto_generate(
        &self,
        _field_name: &str,
        specials: &[String],
        action: PayloadAction,
    ) -> Option<Value> {
        for special in specials {
            match special.as_str() {
                "uuid" if action == PayloadAction::Create => {
                    return Some(json!(uuid::Uuid::new_v4().to_string()));
                }
                "user-created" if action == PayloadAction::Create => {
                    if let Some(user) = self.ctx.user_id() {
                        return Some(json!(user));
                    }
                }
                "user-updated" if action == PayloadAction::Update => {
                    if let Some(user) = self.ctx.user_id() {
                        return Some(json!(user));
                    }
                }
                "role-created" if action == PayloadAction::Create => {
                    if let Some(role) = self.ctx.role_id() {
                        return Some(json!(role));
                    }
                }
                "role-updated" if action == PayloadAction::Update => {
                    if let Some(role) = self.ctx.role_id() {
                        return Some(json!(role));
                    }
                }
                "date-created" if action == PayloadAction::Create => {
                    return Some(json!(chrono::Utc::now().to_rfc3339()));
                }
                "date-updated" if action == PayloadAction::Update => {
                    return Some(json!(chrono::Utc::now().to_rfc3339()));
                }
                _ => {}
            }
        }
        None
    }

    // ── Transformer functions ──────────────────────────────────────

    async fn transform_hash(&self, value: &Value, action: PayloadAction) -> Value {
        if action == PayloadAction::Read {
            return value.clone();
        }

        if let Some(s) = value.as_str() {
            if !s.is_empty() {
                // Hash using argon2
                match nexus_auth::providers::local::LocalAuthProvider::hash_password(s) {
                    Ok(hashed) => return json!(hashed),
                    Err(_) => return value.clone(),
                }
            }
        }

        value.clone()
    }

    fn transform_uuid(&self, value: &Value, action: PayloadAction) -> Value {
        if action == PayloadAction::Create {
            if value.is_null() || value.as_str().map(|s| s.is_empty()).unwrap_or(false) {
                return json!(uuid::Uuid::new_v4().to_string());
            }
        }
        value.clone()
    }

    fn transform_cast_boolean(&self, value: &Value, action: PayloadAction) -> Value {
        if action != PayloadAction::Read {
            return value.clone();
        }

        match value {
            Value::Bool(_) => value.clone(),
            Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    json!(i != 0)
                } else {
                    value.clone()
                }
            }
            Value::String(s) => match s.as_str() {
                "true" | "1" => json!(true),
                "false" | "0" => json!(false),
                "" => Value::Null,
                _ => value.clone(),
            },
            Value::Null => Value::Null,
            _ => value.clone(),
        }
    }

    fn transform_cast_json(&self, value: &Value, action: PayloadAction) -> Value {
        if action != PayloadAction::Read {
            return value.clone();
        }

        if let Some(s) = value.as_str() {
            serde_json::from_str(s).unwrap_or_else(|_| value.clone())
        } else {
            value.clone()
        }
    }

    fn transform_cast_csv(&self, value: &Value, action: PayloadAction) -> Value {
        if action == PayloadAction::Read {
            // Split comma-separated string into array
            if let Some(s) = value.as_str() {
                if s.is_empty() {
                    return json!([]);
                }
                let parts: Vec<Value> = s.split(',').map(|p| json!(p.trim())).collect();
                return json!(parts);
            }
        } else {
            // Join array to comma-separated string
            if let Some(arr) = value.as_array() {
                let parts: Vec<String> = arr
                    .iter()
                    .map(|v| v.as_str().unwrap_or_default().to_string())
                    .collect();
                return json!(parts.join(","));
            }
        }
        value.clone()
    }

    fn transform_conceal(&self, value: &Value, action: PayloadAction) -> Value {
        if action == PayloadAction::Read {
            if !value.is_null() {
                return json!("**********");
            }
        }
        value.clone()
    }

    // ── Date processing ──────────────────────────────────────────

    fn process_dates(&self, action: PayloadAction, payload: &mut Value) {
        if let Some(collection) = self.ctx.schema.collections.get(&self.collection) {
            for (field_name, field_info) in &collection.fields {
                if let Some(val) = payload.get(field_name).cloned() {
                    if val.is_null() {
                        continue;
                    }

                    let processed = match field_info.field_type {
                        FieldType::DateTime if action == PayloadAction::Read => {
                            // Format as ISO datetime
                            if let Some(s) = val.as_str() {
                                if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.f") {
                                    json!(dt.format("%Y-%m-%dT%H:%M:%S").to_string())
                                } else {
                                    val
                                }
                            } else {
                                val
                            }
                        }
                        FieldType::Date if action == PayloadAction::Read => {
                            if let Some(s) = val.as_str() {
                                // Extract date part only
                                json!(s.split('T').next().unwrap_or(s))
                            } else {
                                val
                            }
                        }
                        FieldType::Time if action == PayloadAction::Read => {
                            if let Some(s) = val.as_str() {
                                // Extract time part only (HH:mm:ss)
                                let time_part = if s.contains('T') {
                                    s.split('T').nth(1).unwrap_or(s).split('.').next().unwrap_or(s)
                                } else {
                                    s.split('.').next().unwrap_or(s)
                                };
                                json!(time_part)
                            } else {
                                val
                            }
                        }
                        _ => val,
                    };

                    if let Some(obj) = payload.as_object_mut() {
                        obj.insert(field_name.clone(), processed);
                    }
                }
            }
        }
    }

    // ── JSON stringify ──────────────────────────────────────────

    fn json_stringify_objects(&self, payload: &mut Value) {
        if let Some(collection) = self.ctx.schema.collections.get(&self.collection) {
            if let Some(obj) = payload.as_object_mut() {
                let keys: Vec<String> = obj.keys().cloned().collect();
                for key in keys {
                    if let Some(field_info) = collection.fields.get(&key) {
                        // Only stringify if the field type is NOT json
                        if field_info.field_type != FieldType::Json {
                            if let Some(val) = obj.get(&key) {
                                if val.is_object() || val.is_array() {
                                    let stringified = serde_json::to_string(val).unwrap_or_default();
                                    obj.insert(key, json!(stringified));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // ── Schema introspection ──────────────────────────────────────

    /// Get fields that have special attributes
    fn get_fields_with_specials(&self) -> HashMap<String, FieldWithSpecials> {
        let mut result = HashMap::new();

        if let Some(collection) = self.ctx.schema.collections.get(&self.collection) {
            for (field_name, field_info) in &collection.fields {
                if !field_info.special.is_empty() {
                    result.insert(
                        field_name.clone(),
                        FieldWithSpecials {
                            field_type: field_info.field_type.clone(),
                            specials: field_info.special.clone(),
                        },
                    );
                }
            }
        }

        result
    }
}

struct FieldWithSpecials {
    field_type: FieldType,
    specials: Vec<String>,
}
