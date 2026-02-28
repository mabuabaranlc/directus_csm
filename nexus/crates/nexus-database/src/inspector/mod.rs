//! Schema inspector — introspects database schema at runtime.
//! Queries information_schema (Postgres/MySQL/MSSQL) or sqlite_master (SQLite)
//! and returns a SchemaOverview.

use crate::{DatabaseBackend, DatabaseError, Dialect, SqlValue};
use nexus_types::fields::FieldType;
use nexus_types::schema::{CollectionOverview, FieldOverview, SchemaOverview};
use std::collections::HashMap;
use std::sync::Arc;

/// Introspect the database schema and return a SchemaOverview
pub async fn introspect_schema(
    db: &Arc<dyn DatabaseBackend>,
) -> Result<SchemaOverview, DatabaseError> {
    match db.dialect() {
        Dialect::Postgres | Dialect::CockroachDB => introspect_postgres(db).await,
        Dialect::MySQL => introspect_mysql(db).await,
        Dialect::SQLite => introspect_sqlite(db).await,
        Dialect::MSSQL => introspect_mssql(db).await,
        Dialect::Oracle => Err(DatabaseError::NotSupported(
            "Oracle introspection not yet implemented".to_string(),
        )),
    }
}

async fn introspect_postgres(
    db: &Arc<dyn DatabaseBackend>,
) -> Result<SchemaOverview, DatabaseError> {
    let mut collections: HashMap<String, CollectionOverview> = HashMap::new();

    // Get all tables in the public schema
    let tables_sql = "SELECT table_name FROM information_schema.tables \
                      WHERE table_schema = 'public' AND table_type = 'BASE TABLE' \
                      ORDER BY table_name";
    let tables = db.query(tables_sql, &[]).await?;

    for table_row in &tables {
        let table_name = table_row
            .get("table_name")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();

        if table_name.is_empty() {
            continue;
        }

        // Get columns for this table
        let cols_sql = "SELECT column_name, data_type, column_default, is_nullable, \
                        character_maximum_length, numeric_precision, numeric_scale, \
                        udt_name \
                        FROM information_schema.columns \
                        WHERE table_schema = 'public' AND table_name = $1 \
                        ORDER BY ordinal_position";
        let cols = db
            .query(cols_sql, &[SqlValue::Text(table_name.clone())])
            .await?;

        let mut fields = HashMap::new();
        let mut primary_key = String::new();

        for col in &cols {
            let col_name = col
                .get("column_name")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let data_type = col
                .get("data_type")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let udt_name = col
                .get("udt_name")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let nullable = col
                .get("is_nullable")
                .and_then(|v| v.as_str())
                .unwrap_or("NO")
                == "YES";
            let default_val = col.get("column_default").cloned();
            let precision = col
                .get("numeric_precision")
                .and_then(|v| v.as_i64())
                .map(|n| n as i32);
            let scale = col
                .get("numeric_scale")
                .and_then(|v| v.as_i64())
                .map(|n| n as i32);

            let has_default = default_val
                .as_ref()
                .map(|v| !v.is_null())
                .unwrap_or(false);
            let is_generated = default_val
                .as_ref()
                .and_then(|v| v.as_str())
                .map(|s| s.contains("nextval(") || s.contains("gen_random_uuid"))
                .unwrap_or(false);

            let field_type = pg_type_to_field_type(&data_type, &udt_name);

            fields.insert(
                col_name.clone(),
                FieldOverview {
                    field: col_name.clone(),
                    default_value: if has_default { default_val } else { None },
                    nullable,
                    generated: is_generated,
                    field_type,
                    db_type: Some(udt_name),
                    precision,
                    scale,
                    special: Vec::new(),
                    note: None,
                    validation: None,
                    alias: false,
                    searchable: matches!(
                        pg_type_to_field_type(&data_type, ""),
                        FieldType::String | FieldType::Text
                    ),
                },
            );
        }

        // Get primary key
        let pk_sql = "SELECT kcu.column_name \
                      FROM information_schema.table_constraints tc \
                      JOIN information_schema.key_column_usage kcu \
                        ON tc.constraint_name = kcu.constraint_name \
                        AND tc.table_schema = kcu.table_schema \
                      WHERE tc.constraint_type = 'PRIMARY KEY' \
                        AND tc.table_schema = 'public' \
                        AND tc.table_name = $1 \
                      LIMIT 1";
        let pk_rows = db
            .query(pk_sql, &[SqlValue::Text(table_name.clone())])
            .await?;
        if let Some(pk_row) = pk_rows.first() {
            primary_key = pk_row
                .get("column_name")
                .and_then(|v| v.as_str())
                .unwrap_or("id")
                .to_string();
        }

        if primary_key.is_empty() {
            primary_key = "id".to_string();
        }

        collections.insert(
            table_name.clone(),
            CollectionOverview {
                collection: table_name,
                primary: primary_key,
                fields,
                is_singleton: false,
                accountability: Some("all".to_string()),
            },
        );
    }

    // Get foreign key relations
    let relations = introspect_pg_relations(db).await?;

    Ok(SchemaOverview {
        collections,
        relations,
    })
}

async fn introspect_pg_relations(
    db: &Arc<dyn DatabaseBackend>,
) -> Result<Vec<nexus_types::relations::Relation>, DatabaseError> {
    let sql = "SELECT \
                 tc.table_name AS from_table, \
                 kcu.column_name AS from_column, \
                 ccu.table_name AS to_table, \
                 ccu.column_name AS to_column \
               FROM information_schema.table_constraints AS tc \
               JOIN information_schema.key_column_usage AS kcu \
                 ON tc.constraint_name = kcu.constraint_name \
                 AND tc.table_schema = kcu.table_schema \
               JOIN information_schema.constraint_column_usage AS ccu \
                 ON ccu.constraint_name = tc.constraint_name \
                 AND ccu.table_schema = tc.table_schema \
               WHERE tc.constraint_type = 'FOREIGN KEY' \
                 AND tc.table_schema = 'public'";
    let rows = db.query(sql, &[]).await?;

    let mut relations = Vec::new();
    for row in rows {
        let from_table = row
            .get("from_table")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        let from_col = row
            .get("from_column")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        let to_table = row
            .get("to_table")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        let to_col = row
            .get("to_column")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();

        relations.push(nexus_types::relations::Relation {
            collection: from_table.clone(),
            field: from_col.clone(),
            related_collection: Some(to_table.clone()),
            meta: None,
            schema: Some(nexus_types::relations::ForeignKey {
                table: from_table,
                column: from_col,
                foreign_key_table: to_table,
                foreign_key_column: to_col,
                foreign_key_schema: None,
                constraint_name: None,
                on_update: None,
                on_delete: None,
            }),
        });
    }

    Ok(relations)
}

async fn introspect_mysql(
    db: &Arc<dyn DatabaseBackend>,
) -> Result<SchemaOverview, DatabaseError> {
    let mut collections: HashMap<String, CollectionOverview> = HashMap::new();

    let tables_sql = "SELECT TABLE_NAME as table_name FROM information_schema.TABLES \
                      WHERE TABLE_SCHEMA = DATABASE() AND TABLE_TYPE = 'BASE TABLE'";
    let tables = db.query(tables_sql, &[]).await?;

    for table_row in &tables {
        let table_name = table_row
            .get("table_name")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();

        if table_name.is_empty() {
            continue;
        }

        let cols_sql = "SELECT COLUMN_NAME as column_name, DATA_TYPE as data_type, \
                        COLUMN_DEFAULT as column_default, IS_NULLABLE as is_nullable, \
                        COLUMN_KEY as column_key, EXTRA as extra, \
                        NUMERIC_PRECISION as numeric_precision, NUMERIC_SCALE as numeric_scale \
                        FROM information_schema.COLUMNS \
                        WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = $1 \
                        ORDER BY ORDINAL_POSITION";
        let cols = db
            .query(cols_sql, &[SqlValue::Text(table_name.clone())])
            .await?;

        let mut fields = HashMap::new();
        let mut primary_key = String::new();

        for col in &cols {
            let col_name = col
                .get("column_name")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let data_type = col
                .get("data_type")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let nullable = col
                .get("is_nullable")
                .and_then(|v| v.as_str())
                .unwrap_or("NO")
                == "YES";
            let column_key = col
                .get("column_key")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            let extra = col
                .get("extra")
                .and_then(|v| v.as_str())
                .unwrap_or_default();

            if column_key == "PRI" && primary_key.is_empty() {
                primary_key = col_name.clone();
            }

            let field_type = mysql_type_to_field_type(&data_type);
            let is_generated = extra.contains("auto_increment");

            fields.insert(
                col_name.clone(),
                FieldOverview {
                    field: col_name,
                    default_value: col.get("column_default").cloned(),
                    nullable,
                    generated: is_generated,
                    field_type,
                    db_type: Some(data_type.clone()),
                    precision: col
                        .get("numeric_precision")
                        .and_then(|v| v.as_i64())
                        .map(|n| n as i32),
                    scale: col
                        .get("numeric_scale")
                        .and_then(|v| v.as_i64())
                        .map(|n| n as i32),
                    special: Vec::new(),
                    note: None,
                    validation: None,
                    alias: false,
                    searchable: matches!(
                        mysql_type_to_field_type(&data_type),
                        FieldType::String | FieldType::Text
                    ),
                },
            );
        }

        if primary_key.is_empty() {
            primary_key = "id".to_string();
        }

        collections.insert(
            table_name.clone(),
            CollectionOverview {
                collection: table_name,
                primary: primary_key,
                fields,
                is_singleton: false,
                accountability: Some("all".to_string()),
            },
        );
    }

    Ok(SchemaOverview {
        collections,
        relations: Vec::new(),
    })
}

async fn introspect_sqlite(
    db: &Arc<dyn DatabaseBackend>,
) -> Result<SchemaOverview, DatabaseError> {
    let mut collections: HashMap<String, CollectionOverview> = HashMap::new();

    let tables_sql = "SELECT name FROM sqlite_master WHERE type='table' \
                      AND name NOT LIKE 'sqlite_%' ORDER BY name";
    let tables = db.query(tables_sql, &[]).await?;

    for table_row in &tables {
        let table_name = table_row
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();

        if table_name.is_empty() {
            continue;
        }

        let cols_sql = format!("PRAGMA table_info(\"{}\")", table_name);
        let cols = db.query(&cols_sql, &[]).await?;

        let mut fields = HashMap::new();
        let mut primary_key = String::new();

        for col in &cols {
            let col_name = col
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let col_type = col
                .get("type")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let not_null = col
                .get("notnull")
                .and_then(|v| v.as_i64())
                .unwrap_or(0)
                != 0;
            let is_pk = col.get("pk").and_then(|v| v.as_i64()).unwrap_or(0) != 0;

            if is_pk && primary_key.is_empty() {
                primary_key = col_name.clone();
            }

            let field_type = sqlite_type_to_field_type(&col_type);
            let searchable = matches!(field_type, FieldType::String | FieldType::Text);

            fields.insert(
                col_name.clone(),
                FieldOverview {
                    field: col_name,
                    default_value: col.get("dflt_value").cloned(),
                    nullable: !not_null,
                    generated: is_pk && col_type.to_uppercase().contains("INTEGER"),
                    field_type,
                    db_type: Some(col_type),
                    precision: None,
                    scale: None,
                    special: Vec::new(),
                    note: None,
                    validation: None,
                    alias: false,
                    searchable,
                },
            );
        }

        if primary_key.is_empty() {
            primary_key = "id".to_string();
        }

        collections.insert(
            table_name.clone(),
            CollectionOverview {
                collection: table_name,
                primary: primary_key,
                fields,
                is_singleton: false,
                accountability: Some("all".to_string()),
            },
        );
    }

    Ok(SchemaOverview {
        collections,
        relations: Vec::new(),
    })
}

async fn introspect_mssql(
    db: &Arc<dyn DatabaseBackend>,
) -> Result<SchemaOverview, DatabaseError> {
    let mut collections: HashMap<String, CollectionOverview> = HashMap::new();

    let tables_sql = "SELECT TABLE_NAME as table_name FROM INFORMATION_SCHEMA.TABLES \
                      WHERE TABLE_TYPE = 'BASE TABLE' AND TABLE_SCHEMA = 'dbo'";
    let tables = db.query(tables_sql, &[]).await?;

    for table_row in &tables {
        let table_name = table_row
            .get("table_name")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();

        if table_name.is_empty() {
            continue;
        }

        let cols_sql = "SELECT COLUMN_NAME as column_name, DATA_TYPE as data_type, \
                        IS_NULLABLE as is_nullable, COLUMN_DEFAULT as column_default \
                        FROM INFORMATION_SCHEMA.COLUMNS \
                        WHERE TABLE_NAME = $1 AND TABLE_SCHEMA = 'dbo' \
                        ORDER BY ORDINAL_POSITION";
        let cols = db
            .query(cols_sql, &[SqlValue::Text(table_name.clone())])
            .await?;

        let mut fields = HashMap::new();
        let primary_key = "id".to_string();

        for col in &cols {
            let col_name = col
                .get("column_name")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let data_type = col
                .get("data_type")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let nullable = col
                .get("is_nullable")
                .and_then(|v| v.as_str())
                .unwrap_or("NO")
                == "YES";

            let field_type = mssql_type_to_field_type(&data_type);
            let searchable = matches!(field_type, FieldType::String | FieldType::Text);

            fields.insert(
                col_name.clone(),
                FieldOverview {
                    field: col_name,
                    default_value: col.get("column_default").cloned(),
                    nullable,
                    generated: false,
                    field_type,
                    db_type: Some(data_type),
                    precision: None,
                    scale: None,
                    special: Vec::new(),
                    note: None,
                    validation: None,
                    alias: false,
                    searchable,
                },
            );
        }

        collections.insert(
            table_name.clone(),
            CollectionOverview {
                collection: table_name,
                primary: primary_key,
                fields,
                is_singleton: false,
                accountability: Some("all".to_string()),
            },
        );
    }

    Ok(SchemaOverview {
        collections,
        relations: Vec::new(),
    })
}

// ── Type mapping helpers ─────────────────────────────────────

fn pg_type_to_field_type(data_type: &str, udt_name: &str) -> FieldType {
    let dt = data_type.to_lowercase();
    let udt = udt_name.to_lowercase();

    match dt.as_str() {
        "integer" | "int4" | "smallint" | "int2" | "serial" => FieldType::Integer,
        "bigint" | "int8" | "bigserial" => FieldType::BigInteger,
        "real" | "float4" => FieldType::Float,
        "double precision" | "float8" | "numeric" | "decimal" => FieldType::Decimal,
        "boolean" | "bool" => FieldType::Boolean,
        "character varying" | "varchar" => FieldType::String,
        "text" => FieldType::Text,
        "uuid" => FieldType::Uuid,
        "json" | "jsonb" => FieldType::Json,
        "date" => FieldType::Date,
        "time" | "time without time zone" | "time with time zone" => FieldType::Time,
        "timestamp" | "timestamp without time zone" | "timestamp with time zone" => {
            FieldType::Timestamp
        }
        "bytea" => FieldType::String,
        _ => {
            match udt.as_str() {
                "uuid" => FieldType::Uuid,
                "bool" => FieldType::Boolean,
                "int4" | "int2" | "serial" => FieldType::Integer,
                "int8" | "bigserial" => FieldType::BigInteger,
                "float4" => FieldType::Float,
                "float8" | "numeric" => FieldType::Decimal,
                "json" | "jsonb" => FieldType::Json,
                "timestamptz" | "timestamp" => FieldType::Timestamp,
                _ => FieldType::String,
            }
        }
    }
}

fn mysql_type_to_field_type(data_type: &str) -> FieldType {
    match data_type.to_lowercase().as_str() {
        "int" | "integer" | "tinyint" | "smallint" | "mediumint" => FieldType::Integer,
        "bigint" => FieldType::BigInteger,
        "float" => FieldType::Float,
        "double" | "decimal" | "numeric" => FieldType::Decimal,
        "boolean" | "bool" => FieldType::Boolean,
        "varchar" | "char" => FieldType::String,
        "text" | "tinytext" | "mediumtext" | "longtext" => FieldType::Text,
        "json" => FieldType::Json,
        "date" => FieldType::Date,
        "time" => FieldType::Time,
        "datetime" | "timestamp" => FieldType::DateTime,
        _ => FieldType::String,
    }
}

fn sqlite_type_to_field_type(col_type: &str) -> FieldType {
    let ct = col_type.to_uppercase();
    if ct.contains("INT") {
        FieldType::Integer
    } else if ct.contains("REAL") || ct.contains("FLOAT") || ct.contains("DOUBLE") {
        FieldType::Float
    } else if ct.contains("BOOL") {
        FieldType::Boolean
    } else if ct.contains("TEXT") || ct.contains("CHAR") || ct.contains("CLOB") {
        FieldType::Text
    } else if ct.contains("BLOB") {
        FieldType::String
    } else if ct.contains("DATE") && ct.contains("TIME") {
        FieldType::DateTime
    } else if ct.contains("DATE") {
        FieldType::Date
    } else if ct.contains("TIME") {
        FieldType::Time
    } else if ct.contains("JSON") {
        FieldType::Json
    } else if ct.contains("UUID") {
        FieldType::Uuid
    } else {
        FieldType::String
    }
}

fn mssql_type_to_field_type(data_type: &str) -> FieldType {
    match data_type.to_lowercase().as_str() {
        "int" | "smallint" | "tinyint" => FieldType::Integer,
        "bigint" => FieldType::BigInteger,
        "float" | "real" => FieldType::Float,
        "decimal" | "numeric" | "money" | "smallmoney" => FieldType::Decimal,
        "bit" => FieldType::Boolean,
        "varchar" | "nvarchar" | "char" | "nchar" => FieldType::String,
        "text" | "ntext" => FieldType::Text,
        "date" => FieldType::Date,
        "time" => FieldType::Time,
        "datetime" | "datetime2" | "smalldatetime" | "datetimeoffset" => FieldType::DateTime,
        "uniqueidentifier" => FieldType::Uuid,
        _ => FieldType::String,
    }
}
