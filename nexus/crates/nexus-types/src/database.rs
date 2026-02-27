use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DatabaseDriver {
    Mysql2,
    Pg,
    Cockroachdb,
    Sqlite3,
    Oracledb,
    Mssql,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DatabaseClient {
    Mysql,
    Postgres,
    Cockroachdb,
    Sqlite,
    Oracle,
    Mssql,
    Redshift,
}
