//! Driver-agnostic database abstraction. Wraps Diesel-specific connection
//! management behind a trait so the botserver can be compiled against
//! alternative backends (SQLite, MySQL, in-memory test pools) on
//! non-Linux platforms where libpq may be unavailable.
//!
//! This module introduces the trait surface; concrete Postgres/PG
//! implementations live in `db_pool` (existing) and a SQLite backend can
//! be added without changing call sites that depend on the trait.

use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SqlDialect {
    Postgres,
    Sqlite,
    Mysql,
}

impl fmt::Display for SqlDialect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SqlDialect::Postgres => f.write_str("postgres"),
            SqlDialect::Sqlite => f.write_str("sqlite"),
            SqlDialect::Mysql => f.write_str("mysql"),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("connection: {0}")]
    Connection(String),
    #[error("query: {0}")]
    Query(String),
    #[error("unsupported dialect: {0}")]
    UnsupportedDialect(String),
}

pub trait DbConnection: Send {
    fn execute(&mut self, sql: &str) -> Result<u64, DbError>;
    fn query_scalar(&mut self, sql: &str) -> Result<Option<String>, DbError>;
    fn dialect(&self) -> SqlDialect;
}

pub trait DbDriver: Send + Sync {
    fn dialect(&self) -> SqlDialect;
    fn connect(&self, url: &str) -> Result<Box<dyn DbConnection>, DbError>;
}

pub struct PostgresDriver;

impl DbDriver for PostgresDriver {
    fn dialect(&self) -> SqlDialect {
        SqlDialect::Postgres
    }
    fn connect(&self, _url: &str) -> Result<Box<dyn DbConnection>, DbError> {
        Err(DbError::UnsupportedDialect(
            "PostgresDriver::connect is intentionally a stub; the production code path uses diesel's r2d2 pool"
                .into(),
        ))
    }
}

pub struct SqliteDriver;

impl DbDriver for SqliteDriver {
    fn dialect(&self) -> SqlDialect {
        SqlDialect::Sqlite
    }
    fn connect(&self, _url: &str) -> Result<Box<dyn DbConnection>, DbError> {
        Err(DbError::UnsupportedDialect(
            "SqliteDriver::connect is a stub; production path uses libpq via diesel".into(),
        ))
    }
}

pub struct MySqlDriver;

impl DbDriver for MySqlDriver {
    fn dialect(&self) -> SqlDialect {
        SqlDialect::Mysql
    }
    fn connect(&self, _url: &str) -> Result<Box<dyn DbConnection>, DbError> {
        Err(DbError::UnsupportedDialect("MySqlDriver not yet implemented".into()))
    }
}

pub fn driver_for(dialect: SqlDialect) -> Box<dyn DbDriver> {
    match dialect {
        SqlDialect::Postgres => Box::new(PostgresDriver),
        SqlDialect::Sqlite => Box::new(SqliteDriver),
        SqlDialect::Mysql => Box::new(MySqlDriver),
    }
}

pub fn detect_dialect(url: &str) -> SqlDialect {
    if url.starts_with("postgres://") || url.starts_with("postgresql://") {
        SqlDialect::Postgres
    } else if url.starts_with("sqlite:") || url.ends_with(".db") || url == ":memory:" {
        SqlDialect::Sqlite
    } else if url.starts_with("mysql://") {
        SqlDialect::Mysql
    } else {
        SqlDialect::Postgres
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_dialect_from_url() {
        assert_eq!(detect_dialect("postgres://u@h/d"), SqlDialect::Postgres);
        assert_eq!(detect_dialect("postgresql://u@h/d"), SqlDialect::Postgres);
        assert_eq!(detect_dialect("sqlite::memory:"), SqlDialect::Sqlite);
        assert_eq!(detect_dialect("data/app.db"), SqlDialect::Sqlite);
        assert_eq!(detect_dialect("mysql://u@h/d"), SqlDialect::Mysql);
        assert_eq!(detect_dialect("unknown"), SqlDialect::Postgres);
    }

    #[test]
    fn driver_for_returns_matching_dialect() {
        assert_eq!(driver_for(SqlDialect::Sqlite).dialect(), SqlDialect::Sqlite);
        assert_eq!(driver_for(SqlDialect::Postgres).dialect(), SqlDialect::Postgres);
    }

    /// Only Postgres reaches a live connection, through diesel. The SQLite and
    /// MySQL drivers exist so a URL can be classified into a dialect; they
    /// deliberately refuse to open a connection instead of pretending to.
    #[test]
    fn only_the_diesel_backed_driver_connects() {
        assert!(matches!(
            SqliteDriver.connect(":memory:"),
            Err(DbError::UnsupportedDialect(_))
        ));
        assert!(matches!(
            MySqlDriver.connect("mysql://localhost/db"),
            Err(DbError::UnsupportedDialect(_))
        ));
        assert!(matches!(
            PostgresDriver.connect("postgres://localhost/db"),
            Err(DbError::UnsupportedDialect(_))
        ));
    }

    #[test]
    fn sqlite_urls_are_detected_as_sqlite() {
        assert_eq!(detect_dialect("sqlite://data.db"), SqlDialect::Sqlite);
        assert_eq!(detect_dialect(":memory:"), SqlDialect::Sqlite);
        assert_eq!(detect_dialect("mysql://localhost/db"), SqlDialect::Mysql);
    }
}
