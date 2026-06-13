use thiserror::Error;

#[derive(Error, Debug)]
pub enum ShieldError {
    #[error("scan error: {0}")]
    Scan(String),

    #[error("connection error: {0}")]
    Connection(String),

    #[error("sandbox error: {0}")]
    Sandbox(String),

    #[error("patch error: {0}")]
    Patch(String),

    #[error("verification error: {0}")]
    Verification(String),

    #[error("authentication error: {0}")]
    Auth(String),

    #[error("configuration error: {0}")]
    Config(String),

    #[error("timeout after {0}s")]
    Timeout(u64),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),
}

pub type ShieldResult<T> = Result<T, ShieldError>;
