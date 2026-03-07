use core::fmt::Debug;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OTAError {
    NotSupported,
    NotInitialized,
    AlreadyInProgress,
    NotInProgress,
    WriteFailed,
    VerifyFailed,
    InvalidData,
    StorageFull,
    StorageError,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OTAState {
    Idle,
    Receiving,
    Verifying,
    Ready,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OTAProgress {
    pub received: u32,
    pub total: u32,
    pub state: OTAState,
}

pub trait OTADriver {
    type Error: Debug;

    fn get_state(&self) -> OTAState;

    fn get_progress(&self) -> OTAProgress;

    async fn begin(&mut self, total_size: u32) -> Result<(), Self::Error>;

    async fn write(&mut self, offset: u32, data: &[u8]) -> Result<(), Self::Error>;

    async fn abort(&mut self) -> Result<(), Self::Error>;

    async fn complete(&mut self) -> Result<(), Self::Error>;

    async fn mark_valid(&mut self) -> Result<(), Self::Error>;

    fn get_ota_partition_size(&self) -> u32;
}

pub struct NoOTA;

impl NoOTA {
    pub fn new() -> Self {
        Self
    }
}

impl Default for NoOTA {
    fn default() -> Self {
        Self::new()
    }
}

impl OTADriver for NoOTA {
    type Error = OTAError;

    fn get_state(&self) -> OTAState {
        OTAState::Idle
    }

    fn get_progress(&self) -> OTAProgress {
        OTAProgress {
            received: 0,
            total: 0,
            state: OTAState::Idle,
        }
    }

    async fn begin(&mut self, _total_size: u32) -> Result<(), Self::Error> {
        Err(OTAError::NotSupported)
    }

    async fn write(&mut self, _offset: u32, _data: &[u8]) -> Result<(), Self::Error> {
        Err(OTAError::NotSupported)
    }

    async fn abort(&mut self) -> Result<(), Self::Error> {
        Err(OTAError::NotSupported)
    }

    async fn complete(&mut self) -> Result<(), Self::Error> {
        Err(OTAError::NotSupported)
    }

    async fn mark_valid(&mut self) -> Result<(), Self::Error> {
        Err(OTAError::NotSupported)
    }

    fn get_ota_partition_size(&self) -> u32 {
        0
    }
}
