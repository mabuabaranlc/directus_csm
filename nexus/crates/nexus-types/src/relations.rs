use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForeignKey {
    pub table: String,
    pub column: String,
    pub foreign_key_table: String,
    pub foreign_key_column: String,
    pub foreign_key_schema: Option<String>,
    pub constraint_name: Option<String>,
    pub on_update: Option<ForeignKeyAction>,
    pub on_delete: Option<ForeignKeyAction>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ForeignKeyAction {
    #[serde(rename = "NO ACTION")]
    NoAction,
    #[serde(rename = "RESTRICT")]
    Restrict,
    #[serde(rename = "CASCADE")]
    Cascade,
    #[serde(rename = "SET NULL")]
    SetNull,
    #[serde(rename = "SET DEFAULT")]
    SetDefault,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationMeta {
    pub id: Option<i64>,
    pub many_collection: String,
    pub many_field: String,
    pub one_collection: Option<String>,
    pub one_field: Option<String>,
    pub one_collection_field: Option<String>,
    pub one_allowed_collections: Option<Vec<String>>,
    #[serde(default = "default_deselect_action")]
    pub one_deselect_action: String,
    pub junction_field: Option<String>,
    pub sort_field: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<bool>,
}

fn default_deselect_action() -> String {
    "nullify".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relation {
    pub collection: String,
    pub field: String,
    pub related_collection: Option<String>,
    pub schema: Option<ForeignKey>,
    pub meta: Option<RelationMeta>,
}
