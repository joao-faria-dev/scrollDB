use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]

pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Invalid file format: expected {expected}, found {found}")]
    InvalidFormatError { expected: String, found: String },

    #[error("Corrupted database: {reason}")]
    CorruptedDatabase { reason: String },

    #[error("Database is already open")]
    DatabaseAlreadyOpen,

    #[error("Database is not open")]
    DatabaseNotOpen,
}
