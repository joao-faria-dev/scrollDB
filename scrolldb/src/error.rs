use thiserror::Error;

/// Result type alias for ScrollDB operations
pub type Result<T> = std::result::Result<T, Error>;

/// Error types for ScrollDB operations
#[derive(Error, Debug)]
pub enum Error {
    /// I/O errors
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Invalid file format
    #[error("Invalid file format: expected {expected}, found {found}")]
    InvalidFileFormat { expected: String, found: String },

    /// Corrupted database
    #[error("Corrupted database: {reason}")]
    CorruptedDatabase { reason: String },

    /// Database is already open
    #[error("Database is already open")]
    DatabaseAlreadyOpen,

    /// Database is not open
    #[error("Database is not open")]
    DatabaseNotOpen,
}
