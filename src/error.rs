//! Error types for bonrec-rs

use std::path::PathBuf;
use thiserror::Error;

/// Main error type for bonrec-rs operations
#[derive(Error, Debug)]
pub enum BonrecError {
    /// Failed to load BonDriver DLL
    #[error("Failed to load BonDriver: {path}")]
    DllLoadError {
        path: PathBuf,
        #[source]
        source: libloading::Error,
    },

    /// BonDriver CreateBonDriver returned null
    #[error("CreateBonDriver returned null for: {path}")]
    CreateBonDriverFailed { path: PathBuf },

    /// Failed to open tuner
    #[error("Failed to open tuner")]
    TunerOpenFailed,

    /// Tuner is already in use
    #[error("Tuner is already in use or unavailable")]
    TunerBusy,

    /// Failed to set channel
    #[error("Failed to set channel: space={space}, channel={channel}")]
    SetChannelFailed { space: u32, channel: u32 },

    /// Channel not found
    #[error("Channel not found: space={space}, channel={channel}")]
    ChannelNotFound { space: u32, channel: u32 },

    /// TS stream timeout
    #[error("TS stream timeout after {timeout_ms}ms")]
    StreamTimeout { timeout_ms: u32 },

    /// Weak signal
    #[error("Weak signal level: {level:.2}dB")]
    WeakSignal { level: f32 },

    /// No signal
    #[error("No signal received")]
    NoSignal,

    /// SI information parse error
    #[error("Failed to parse SI information: {message}")]
    SiParseError { message: String },

    /// Database error
    #[error("Database error: {0}")]
    DatabaseError(#[from] rusqlite::Error),

    /// IO error
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// JSON serialization error
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    /// Invalid argument
    #[error("Invalid argument: {message}")]
    InvalidArgument { message: String },

    /// BonDriver file not found
    #[error("BonDriver file not found: {path}")]
    BonDriverNotFound { path: PathBuf },

    /// Tuning space not found
    #[error("Tuning space not found: {name}")]
    TuningSpaceNotFound { name: String },

    /// User interrupted operation
    #[error("Operation interrupted by user")]
    Interrupted,

    /// Generic operation error
    #[error("{message}")]
    OperationError { message: String },
}

/// Result type alias for bonrec operations
pub type Result<T> = std::result::Result<T, BonrecError>;

impl BonrecError {
    /// Create a new DLL load error
    pub fn dll_load_error(path: impl Into<PathBuf>, source: libloading::Error) -> Self {
        BonrecError::DllLoadError {
            path: path.into(),
            source,
        }
    }

    /// Create a new invalid argument error
    pub fn invalid_argument(message: impl Into<String>) -> Self {
        BonrecError::InvalidArgument {
            message: message.into(),
        }
    }

    /// Create a new operation error
    pub fn operation_error(message: impl Into<String>) -> Self {
        BonrecError::OperationError {
            message: message.into(),
        }
    }

    /// Create a new SI parse error
    pub fn si_parse_error(message: impl Into<String>) -> Self {
        BonrecError::SiParseError {
            message: message.into(),
        }
    }

    /// Get exit code for this error
    pub fn exit_code(&self) -> i32 {
        match self {
            BonrecError::DllLoadError { .. }
            | BonrecError::CreateBonDriverFailed { .. }
            | BonrecError::BonDriverNotFound { .. } => 2,

            BonrecError::TunerOpenFailed | BonrecError::TunerBusy => 3,

            BonrecError::Interrupted => 4,

            _ => 1,
        }
    }
}
