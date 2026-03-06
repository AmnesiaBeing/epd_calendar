use core::fmt::Debug;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OTAError {
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