pub type SystemResult<T> = core::result::Result<T, SystemError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum SystemError {
    HardwareError(HardwareError),
    ServiceError(ServiceError),
    StorageError(StorageError),
    NetworkError(NetworkError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum HardwareError {
    NotInitialized,
    InvalidParameter,
    Timeout,
    CommunicationError,
    PowerError,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum ServiceError {
    NotInitialized,
    InvalidState,
    Timeout,
    OperationFailed,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum StorageError {
    NotFound,
    Corrupted,
    WriteFailed,
    ReadFailed,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum NetworkError {
    NotConnected,
    Timeout,
    AuthenticationFailed,
    ServerError,
    Unknown,
}
