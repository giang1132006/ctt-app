//! Typed errors for profile serialization, disk I/O, and schema validation.

use std::fmt;

use crate::domain::validation::MappingError;

/// Errors returned when loading or saving profiles.
#[derive(Debug)]
pub enum StorageError {
    Io(std::io::Error),
    UnsupportedSchemaVersion { found: u32, expected: u32 },
    ParseError(String),
    ValidationFailed(Vec<MappingError>),
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(err) => write!(f, "I/O error: {err}"),
            Self::UnsupportedSchemaVersion { found, expected } => write!(
                f,
                "Unsupported profile schema version {found} (expected {expected})"
            ),
            Self::ParseError(msg) => write!(f, "Profile parse error: {msg}"),
            Self::ValidationFailed(errors) => {
                write!(
                    f,
                    "Profile validation failed with {} error(s)",
                    errors.len()
                )
            }
        }
    }
}

impl std::error::Error for StorageError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<std::io::Error> for StorageError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}
