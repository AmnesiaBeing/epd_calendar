#![allow(async_fn_in_trait)]

use serde::{Deserialize, Serialize};

/// BLE 状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BleState {
    Disconnected,
    Advertising,
    Connected,
    Pairing,
}

/// 配置段
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConfigSection {
    Network,
    Time,
    Display,
    Power,
    Log,
}

/// BLE 指令 (外部 → 设备)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BleCommand {
    // 配置查询/设置
    GetConfig { section: ConfigSection },
    SetConfig { section: ConfigSection, data: String },

    // 时间同步
    SyncTime { timestamp: i64 },

    // 网络操作
    SetWiFi { ssid: String, password: String },
    TestWiFi,

    // 设备控制
    RefreshWeather,
    ForceRefresh,
    GetStatus,

    // OTA
    StartOta,
    OtaFirmware { chunk: Vec<u8>, offset: u32 },
    FinishOta,
}

/// BLE 响应 (设备 → 外部)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BleResponse {
    pub success: bool,
    pub message: String,
    pub data: Option<String>,
}

/// BLE Service Trait
pub trait BleService: Send + Sync {
    type Error;

    async fn init(&self) -> Result<(), Self::Error>;
    async fn start_advertise(&self) -> Result<(), Self::Error>;
    async fn stop_advertise(&self) -> Result<(), Self::Error>;
    async fn wait_for_connection(&self) -> Result<(), Self::Error>;
    async fn disconnect(&self) -> Result<(), Self::Error>;
    fn get_state(&self) -> BleState;

    /// 处理 BLE 命令
    async fn handle_command(&self, cmd: BleCommand) -> Result<BleResponse, Self::Error>;

    /// 轮询命令 (设备侧)
    async fn poll_command(&self) -> Option<BleCommand>;

    /// 推送响应 (设备侧)
    async fn push_response(&self, resp: BleResponse) -> Result<(), Self::Error>;
}

/// 空实现 (用于无 BLE 平台)
pub struct NoBle;

impl NoBle {
    pub fn new() -> Self {
        Self
    }
}

impl BleService for NoBle {
    type Error = core::convert::Infallible;

    async fn init(&self) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn start_advertise(&self) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn stop_advertise(&self) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn wait_for_connection(&self) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn disconnect(&self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn get_state(&self) -> BleState {
        BleState::Disconnected
    }

    async fn handle_command(&self, _cmd: BleCommand) -> Result<BleResponse, Self::Error> {
        Ok(BleResponse {
            success: false,
            message: "BLE not available".to_string(),
            data: None,
        })
    }

    async fn poll_command(&self) -> Option<BleCommand> {
        None
    }

    async fn push_response(&self, _resp: BleResponse) -> Result<(), Self::Error> {
        Ok(())
    }
}
