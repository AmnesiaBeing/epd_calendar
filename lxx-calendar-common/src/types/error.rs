pub type SystemResult<T> = core::result::Result<T, SystemError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemError {
    HardwareError(HardwareError),
    ServiceError(ServiceError),
    StorageError(StorageError),
    NetworkError(NetworkError),
}

impl core::fmt::Display for SystemError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            SystemError::HardwareError(e) => write!(f, "Hardware error: {:?}", e),
            SystemError::ServiceError(e) => write!(f, "Service error: {:?}", e),
            SystemError::StorageError(e) => write!(f, "Storage error: {:?}", e),
            SystemError::NetworkError(e) => write!(f, "Network error: {:?}", e),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HardwareError {
    NotInitialized,
    InvalidParameter,
    Timeout,
    CommunicationError,
    PowerError,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceError {
    NotInitialized,
    InvalidState,
    Timeout,
    OperationFailed,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageError {
    NotFound,
    Corrupted,
    WriteFailed,
    ReadFailed,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkError {
    NotConnected,
    Timeout,
    AuthenticationFailed,
    ServerError,
    Unknown,
}
