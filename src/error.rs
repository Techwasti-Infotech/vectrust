use thiserror::Error;

/// The central error type for all Vectrust operations.
#[derive(Error, Debug)]
pub enum VectrustError {
    #[error("Dimension mismatch: expected {expected}, got {got}")]
    DimensionMismatch { expected: usize, got: usize },

    #[error("Collection '{0}' already exists")]
    CollectionAlreadyExists(String),

    #[error("Collection '{0}' not found")]
    CollectionNotFound(String),

    #[error("Point with ID '{0}' not found")]
    PointNotFound(String),

    #[error("Vector contains invalid values (NaN or Infinity)")]
    InvalidVectorData,

    #[error("Index configuration error: {0}")]
    ConfigError(String),

    #[error("Storage I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Serialization / Deserialization error: {0}")]
    SerializationError(#[from] bincode::Error),

    #[error("JSON serialization error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Storage corruption detected: {0}")]
    CorruptedData(String),

    #[error("Lock acquisition error: {0}")]
    LockError(String),

    #[error("Filter evaluation error: {0}")]
    FilterError(String),

    #[error("Internal engine error: {0}")]
    Internal(String),
}

/// A specialized Result type for Vectrust operations.
pub type Result<T> = std::result::Result<T, VectrustError>;
