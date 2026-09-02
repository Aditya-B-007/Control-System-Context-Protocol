use thiserror::Error;

/// C-compatible status codes for C-ABI exports
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum CscpStatusCode {
    Success = 0,
    InvalidMagic = -1,
    AbiVersionMismatch = -2,
    NullPointer = -3,
    BufferTooSmall = -4,
    ShmAccessError = -5,
    PoisonedLock = -6,
    QueueFull = -7,
    ShmOpenFailed = -8,
    ShmMapFailed = -9,
    RateLimited = -10,
    UnknownError = -99,
}

#[derive(Error, Debug)]
pub enum CscpError {
    #[error("Invalid SHM Magic Signature: expected 0x43534350, found 0x{0:X}")]
    InvalidMagic(u32),

    #[error("ABI Version Mismatch: expected 0x{expected:X}, found 0x{actual:X}")]
    AbiVersionMismatch { expected: u32, actual: u32 },

    #[error("Null pointer provided to C-ABI interface: {0}")]
    NullPointer(&'static str),

    #[error("Buffer length {provided} is smaller than required dimension {required}")]
    BufferTooSmall { provided: usize, required: usize },

    #[error("Shared memory access error: {0}")]
    ShmAccessError(String),

    #[error("Queue is full, frame dropped")]
    QueueFull,

    #[error("Failed to open shared memory region: {0}")]
    ShmOpenFailed(String),

    #[error("Failed to map shared memory region: {0}")]
    ShmMapFailed(String),

    #[error("Rate limited by leaky bucket")]
    RateLimited,
}

impl From<CscpError> for CscpStatusCode {
    fn from(err: CscpError) -> Self {
        match err {
            CscpError::InvalidMagic(_) => CscpStatusCode::InvalidMagic,
            CscpError::AbiVersionMismatch { .. } => CscpStatusCode::AbiVersionMismatch,
            CscpError::NullPointer(_) => CscpStatusCode::NullPointer,
            CscpError::BufferTooSmall { .. } => CscpStatusCode::BufferTooSmall,
            CscpError::ShmAccessError(_) => CscpStatusCode::ShmAccessError,
            CscpError::QueueFull => CscpStatusCode::QueueFull,
            CscpError::ShmOpenFailed(_) => CscpStatusCode::ShmOpenFailed,
            CscpError::ShmMapFailed(_) => CscpStatusCode::ShmMapFailed,
            CscpError::RateLimited => CscpStatusCode::RateLimited,
        }
    }
}
