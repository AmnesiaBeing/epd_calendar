pub type SystemResult<T> = core::result::Result<T, SystemError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum SystemError {
    HardwareError(HardwareError),
    ServiceError(ServiceError),
    StorageError(StorageError),
    NetworkError(NetworkError),
    DataError(DataError),
    ButtonTaskError,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum DataError {
    NotFound,
    Corrupted,
    ParseError,
    Unknown,
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

impl From<HardwareError> for SystemError {
    fn from(value: HardwareError) -> Self {
        Self::HardwareError(value)
    }
}

impl From<ServiceError> for SystemError {
    fn from(value: ServiceError) -> Self {
        Self::ServiceError(value)
    }
}

impl From<StorageError> for SystemError {
    fn from(value: StorageError) -> Self {
        Self::StorageError(value)
    }
}

impl From<NetworkError> for SystemError {
    fn from(value: NetworkError) -> Self {
        Self::NetworkError(value)
    }
}
