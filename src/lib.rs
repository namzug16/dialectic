//! Dialectic's core: pull requests stored in SQLite, branches read and merged with the `git` CLI.
//! The GUI (`src/ui`) and the terminal CLI both go through this crate, so they share every rule.

pub mod db;

use std::fmt;

/// Every core failure is a message a person (or an agent) can act on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error(pub String);

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

impl From<rusqlite::Error> for Error {
    fn from(error: rusqlite::Error) -> Self {
        Error(format!("database error: {error}"))
    }
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Error(format!("I/O error: {error}"))
    }
}

pub type Result<T> = std::result::Result<T, Error>;
