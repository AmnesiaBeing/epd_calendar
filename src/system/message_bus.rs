use embassy_sync::channel::Channel;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use heapless::String;

const MESSAGE_CHANNEL_SIZE: usize = 16;

pub enum SystemMessage {
    DisplayRefresh,
    DataUpdated(String<64>),
    NetworkConnected,
    NetworkDisconnected,
    BatteryLow,
    Error(String<64>),
}

pub struct MessageBus {
    sender: Channel<CriticalSectionRawMutex, SystemMessage, MESSAGE_CHANNEL_SIZE>,
}

impl MessageBus {
    pub fn new() -> Self {
        Self {
            sender: Channel::new(),
        }
    }

    pub fn sender(&self) -> &Channel<CriticalSectionRawMutex, SystemMessage, MESSAGE_CHANNEL_SIZE> {
        &self.sender
    }

    pub fn receiver(&self) -> &Channel<CriticalSectionRawMutex, SystemMessage, MESSAGE_CHANNEL_SIZE> {
        &self.sender
    }

    pub async fn send(&self, message: SystemMessage) {
        self.sender.send(message).await;
    }

    pub async fn receive(&self) -> SystemMessage {
        self.sender.receive().await
    }
}

impl Default for MessageBus {
    fn default() -> Self {
        Self::new()
    }
}
