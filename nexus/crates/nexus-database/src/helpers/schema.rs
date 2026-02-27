use crate::{DatabaseBackend, DatabaseError, Dialect};
use nexus_types::fields::FieldType;
use nexus_types::relations::Relation;
use nexus_types::schema::{CollectionOverview, FieldOverview, SchemaOverview};
use std::collections::HashMap;

/// Database schema inspector — introspects the database at runtime
/// to build a SchemaOverview used by all services
pub struct SchemaInspector<'a> {
    db: &'a dyn DatabaseBackend,
}

impl<'a> SchemaInspector<'a> {
    pub fn new(db: &'a dyn DatabaseBackend) -> Self {
        Self { db }
    }

    /// Convenience static method: build a snapshot of the schema
    pub async fn snapshot(db: &'a dyn DatabaseBackend) -> Result<SchemaOverview, DatabaseError> {
        let inspector = Self::new(db);
        inspector.overview().await
    }

    /// Build a complete schema overview from the database
    pub async fn overview(&self) -> Result<SchemaOverview, DatabaseError> {
        let tables = self.get_tables().await?;
        let columns = self.get_columns().await?;
        let foreign_keys = self.get_foreign_keys().await?;

        let mut collections = HashMap::new();

        for table in &tables {
            let table_columns = columns
                .iter()
                .filter(|c| c.table_name == table.name)
                .collect::<Vec<_>>();

            let primary = table_columns
                .iter()
                .find(|c| c.is_primary_key)
                .map(|c| c.column_name.clone())
                .unwrap_or_else(|| "id".to_string());

            let mut fields = HashMap::new();
            for col in &table_columns {
                fields.insert(
                    col.column_name.clone(),
                    FieldOverview {
                        field: col.column_name.clone(),
                        default_value: col.default_value.clone(),
                        nullable: col.is_nullable,
                        generated: col.is_generated,
                        field_type: map_db_type_to_field_type(&col.data_type, self.db.dialect()),
                        db_type: Some(col.data_type.clone()),
                        precision: col.numeric_precision,
                        scale: col.numeric_scale,
                        special: Vec::new(),
                        note: None,
                        validation: None,
                        alias: false,
                        searchable: matches!(
                            map_db_type_to_field_type(&col.data_type, self.db.dialect()),
                            FieldType::String | FieldType::Text
                        ),
                    },
                );
            }

            collections.insert(
                table.name.clone(),
                CollectionOverview {
                    collection: table.name.clone(),
                    primary,
                    fields,
                    is_singleton: false,
                    accountability: Some("all".to_string()),
                },
            );
        }

        let relations = foreign_keys
            .iter()
            .map(|fk| Relation {
                collection: fk.table_name.clone(),
                field: fk.column_name.clone(),
                related_collection: Some(fk.foreign_table_name.clone()),
                schema: Some(nexus_types::relations::ForeignKey {
                    table: fk.table_name.clone(),
                    column: fk.column_name.clone(),
                    foreign_key_table: fk.foreign_table_name.clone(),
                    foreign_key_column: fk.foreign_column_name.clone(),
                    foreign_key_schema: None,
                    constraint_name: fk.constraint_name.clone(),
                    on_update: None,
                    on_delete: None,
                }),
                meta: None,
            })
            .collect();

        Ok(SchemaOverview {
            collections,
            relations,
        })
    }

    async fn get_tables(&self) -> Result<Vec<TableInfo>, DatabaseError> {
        let sql = match self.db.dialect() {
            Dialect::Postgres | Dialect::CockroachDB => {
                "SELECT table_name as name FROM information_schema.tables \
                 WHERE table_schema = 'public' AND table_type = 'BASE TABLE' \
                 ORDER BY table_name"
            }
            Dialect::MySQL => {
                "SELECT table_name as name FROM information_schema.tables \
                 WHERE table_schema = DATABASE() AND table_type = 'BASE TABLE' \
                 ORDER BY table_name"
            }
            Dialect::SQLite => {
                "SELECT name FROM sqlite_master \
                 WHERE type = 'table' AND name NOT LIKE 'sqlite_%' \
                 ORDER BY name"
            }
            Dialect::MSSQL => {
                "SELECT TABLE_NAME as name FROM INFORMATION_SCHEMA.TABLES \
                 WHERE TABLE_TYPE = 'BASE TABLE' ORDER BY TABLE_NAME"
            }
            Dialect::Oracle => {
                "SELECT table_name as name FROM user_tables ORDER BY table_name"
            }
        };

        let rows = self.db.query(sql, &[]).await?;

        Ok(rows
            .into_iter()
            .map(|row| TableInfo {
                name: row
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string(),
            })
            .collect())
    }

    async fn get_columns(&self) -> Result<Vec<ColumnInfo>, DatabaseError> {
        let sql = match self.db.dialect() {
            Dialect::Postgres | Dialect::CockroachDB => {
                "SELECT c.table_name, c.column_name, c.data_type, c.column_default as default_value, \
                 CASE WHEN c.is_nullable = 'YES' THEN 'true' ELSE 'false' END as is_nullable, \
                 CASE WHEN c.is_generated != 'NEVER' THEN 'true' ELSE 'false' END as is_generated, \
                 c.numeric_precision::text, c.numeric_scale::text, \
                 CASE WHEN pk.column_name IS NOT NULL THEN 'true' ELSE 'false' END as is_primary_key \
                 FROM information_schema.columns c \
                 LEFT JOIN ( \
                   SELECT kcu.table_name, kcu.column_name \
                   FROM information_schema.table_constraints tc \
                   JOIN information_schema.key_column_usage kcu ON tc.constraint_name = kcu.constraint_name \
                   WHERE tc.constraint_type = 'PRIMARY KEY' AND tc.table_schema = 'public' \
                 ) pk ON c.table_name = pk.table_name AND c.column_name = pk.column_name \
                 WHERE c.table_schema = 'public' \
                 ORDER BY c.table_name, c.ordinal_position"
            }
            Dialect::SQLite => {
                "SELECT '' as table_name, '' as column_name, '' as data_type, \
                 NULL as default_value, 'false' as is_nullable, 'false' as is_generated, \
                 NULL as numeric_precision, NULL as numeric_scale, 'false' as is_primary_key \
                 WHERE 0"
            }
            _ => {
                "SELECT c.TABLE_NAME as table_name, c.COLUMN_NAME as column_name, \
                 c.DATA_TYPE as data_type, c.COLUMN_DEFAULT as default_value, \
                 CASE WHEN c.IS_NULLABLE = 'YES' THEN 'true' ELSE 'false' END as is_nullable, \
                 'false' as is_generated, \
                 CAST(c.NUMERIC_PRECISION AS CHAR) as numeric_precision, \
                 CAST(c.NUMERIC_SCALE AS CHAR) as numeric_scale, \
                 CASE WHEN pk.COLUMN_NAME IS NOT NULL THEN 'true' ELSE 'false' END as is_primary_key \
                 FROM INFORMATION_SCHEMA.COLUMNS c \
                 LEFT JOIN ( \
                   SELECT ku.TABLE_NAME, ku.COLUMN_NAME \
                   FROM INFORMATION_SCHEMA.TABLE_CONSTRAINTS tc \
                   JOIN INFORMATION_SCHEMA.KEY_COLUMN_USAGE ku ON tc.CONSTRAINT_NAME = ku.CONSTRAINT_NAME \
                   WHERE tc.CONSTRAINT_TYPE = 'PRIMARY KEY' \
                 ) pk ON c.TABLE_NAME = pk.TABLE_NAME AND c.COLUMN_NAME = pk.COLUMN_NAME \
                 ORDER BY c.TABLE_NAME, c.ORDINAL_POSITION"
            }
        };

        let rows = self.db.query(sql, &[]).await?;

        Ok(rows
            .into_iter()
            .map(|row| {
                let str_bool = |key: &str| -> bool {
                    row.get(key)
                        .and_then(|v| v.as_str())
                        .map(|s| s == "true")
                        .unwrap_or(false)
                };

                ColumnInfo {
                    table_name: row.get("table_name").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
                    column_name: row.get("column_name").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
                    data_type: row.get("data_type").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
                    default_value: row.get("default_value").cloned(),
                    is_nullable: str_bool("is_nullable"),
                    is_generated: str_bool("is_generated"),
                    is_primary_key: str_bool("is_primary_key"),
                    numeric_precision: row.get("numeric_precision").and_then(|v| v.as_str()).and_then(|s| s.parse().ok()),
                    numeric_scale: row.get("numeric_scale").and_then(|v| v.as_str()).and_then(|s| s.parse().ok()),
                }
            })
            .collect())
    }

    async fn get_foreign_keys(&self) -> Result<Vec<ForeignKeyInfo>, DatabaseError> {
        let sql = match self.db.dialect() {
            Dialect::Postgres | Dialect::CockroachDB => {
                "SELECT kcu.table_name, kcu.column_name, \
                 ccu.table_name as foreign_table_name, ccu.column_name as foreign_column_name, \
                 tc.constraint_name \
                 FROM information_schema.table_constraints tc \
                 JOIN information_schema.key_column_usage kcu ON tc.constraint_name = kcu.constraint_name \
                 JOIN information_schema.constraint_column_usage ccu ON ccu.constraint_name = tc.constraint_name \
                 WHERE tc.constraint_type = 'FOREIGN KEY' AND tc.table_schema = 'public'"
            }
            Dialect::SQLite => {
                "SELECT '' as table_name, '' as column_name, '' as foreign_table_name, \
                 '' as foreign_column_name, '' as constraint_name WHERE 0"
            }
            _ => {
                "SELECT kcu.TABLE_NAME as table_name, kcu.COLUMN_NAME as column_name, \
                 kcu.REFERENCED_TABLE_NAME as foreign_table_name, \
                 kcu.REFERENCED_COLUMN_NAME as foreign_column_name, \
                 kcu.CONSTRAINT_NAME as constraint_name \
                 FROM INFORMATION_SCHEMA.KEY_COLUMN_USAGE kcu \
                 WHERE kcu.REFERENCED_TABLE_NAME IS NOT NULL"
            }
        };

        let rows = self.db.query(sql, &[]).await?;

        Ok(rows
            .into_iter()
            .map(|row| ForeignKeyInfo {
                table_name: row.get("table_name").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
                column_name: row.get("column_name").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
                foreign_table_name: row.get("foreign_table_name").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
                foreign_column_name: row.get("foreign_column_name").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
                constraint_name: row.get("constraint_name").and_then(|v| v.as_str()).map(|s| s.to_string()),
            })
            .collect())
    }
}

#[derive(Debug)]
struct TableInfo {
    name: String,
}

#[derive(Debug)]
struct ColumnInfo {
    table_name: String,
    column_name: String,
    data_type: String,
    default_value: Option<serde_json::Value>,
    is_nullable: bool,
    is_generated: bool,
    is_primary_key: bool,
    numeric_precision: Option<i32>,
    numeric_scale: Option<i32>,
}

#[derive(Debug)]
struct ForeignKeyInfo {
    table_name: String,
    column_name: String,
    foreign_table_name: String,
    foreign_column_name: String,
    constraint_name: Option<String>,
}

/// Map database column types to Nexus field types
pub fn map_db_type_to_field_type(db_type: &str, _dialect: Dialect) -> FieldType {
    let t = db_type.to_lowercase();

    match t.as_str() {
        "int" | "integer" | "int4" | "mediumint" => FieldType::Integer,
        "bigint" | "int8" => FieldType::BigInteger,
        "smallint" | "int2" | "tinyint" => FieldType::Integer,
        "serial" | "serial4" => FieldType::Integer,
        "bigserial" | "serial8" => FieldType::BigInteger,
        "float" | "float4" | "real" => FieldType::Float,
        "double" | "double precision" | "float8" => FieldType::Float,
        "decimal" | "numeric" | "money" => FieldType::Decimal,
        "boolean" | "bool" | "bit" => FieldType::Boolean,
        "varchar" | "character varying" | "nvarchar" | "char" | "character" | "nchar" => FieldType::String,
        "text" | "mediumtext" | "longtext" | "tinytext" | "ntext" | "clob" => FieldType::Text,
        "date" => FieldType::Date,
        "time" | "time without time zone" | "time with time zone" => FieldType::Time,
        "datetime" | "datetime2" | "smalldatetime" => FieldType::DateTime,
        "timestamp" | "timestamp without time zone" | "timestamp with time zone" | "timestamptz" => FieldType::Timestamp,
        "json" | "jsonb" => FieldType::Json,
        "uuid" | "uniqueidentifier" => FieldType::Uuid,
        "bytea" | "binary" | "varbinary" | "blob" | "mediumblob" | "longblob" | "image" => FieldType::Binary,
        "geometry" | "geography" => FieldType::Geometry,
        "point" => FieldType::GeometryPoint,
        "linestring" => FieldType::GeometryLineString,
        "polygon" => FieldType::GeometryPolygon,
        "multipoint" => FieldType::GeometryMultiPoint,
        "multilinestring" => FieldType::GeometryMultiLineString,
        "multipolygon" => FieldType::GeometryMultiPolygon,
        _ => FieldType::Unknown,
    }
}
