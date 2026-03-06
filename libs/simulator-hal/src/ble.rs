//! Simulated BLE Service

use lxx_calendar_common::traits::ble::{
    BleService, BleState, BleCommand, BleResponse, ConfigSection,
};
use crate::{SimulatorConfig, SimulatedWifi};
use tokio::sync::{mpsc, RwLock};
use std::sync::Arc;

/// Simulated BLE
pub struct SimulatedBle {
    state: Arc<RwLock<BleState>>,
    cmd_tx: mpsc::Sender<BleCommand>,
    cmd_rx: Arc<RwLock<mpsc::Receiver<BleCommand>>>,
    resp_tx: mpsc::Sender<BleResponse>,
    config: Arc<SimulatorConfig>,
    wifi: Arc<SimulatedWifi>,
}

impl SimulatedBle {
    pub fn new(config: Arc<SimulatorConfig>, wifi: Arc<SimulatedWifi>) -> Arc<Self> {
        let (cmd_tx, cmd_rx) = mpsc::channel(32);
        let (resp_tx, resp_rx) = mpsc::channel(32);

        Arc::new(Self {
            state: Arc::new(RwLock::new(BleState::Disconnected)),
            cmd_tx,
            cmd_rx: Arc::new(RwLock::new(cmd_rx)),
            resp_tx,
            config,
            wifi,
        })
    }

    /// 外部注入命令 (通过 HTTP API)
    pub async fn inject_command(&self, cmd: BleCommand) -> Result<BleResponse, std::io::Error> {
        self.cmd_tx.send(cmd).await
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::BrokenPipe, e))?;

        // 简化处理：立即返回接收确认
        Ok(BleResponse {
            success: true,
            message: "Command received".to_string(),
            data: None,
        })
    }

    /// 获取响应 (供外部查询)
    pub async fn get_response(&self) -> Option<BleResponse> {
        self.resp_tx.send(BleResponse {
            success: true,
            message: "Response placeholder".to_string(),
            data: None,
        }).await.ok();
        None // 简化实现
    }
}

impl BleService for SimulatedBle {
    type Error = std::io::Error;

    async fn init(&self) -> Result<(), Self::Error> {
        log::info!("BLE initialized");
        Ok(())
    }

    async fn start_advertise(&self) -> Result<(), Self::Error> {
        *self.state.write().await = BleState::Advertising;
        log::info!("BLE advertising started");
        Ok(())
    }

    async fn stop_advertise(&self) -> Result<(), Self::Error> {
        *self.state.write().await = BleState::Disconnected;
        Ok(())
    }

    async fn wait_for_connection(&self) -> Result<(), Self::Error> {
        *self.state.write().await = BleState::Connected;
        log::info!("BLE connected");
        Ok(())
    }

    async fn disconnect(&self) -> Result<(), Self::Error> {
        *self.state.write().await = BleState::Disconnected;
        log::info!("BLE disconnected");
        Ok(())
    }

    fn get_state(&self) -> BleState {
        // 简化：实际应该异步读取
        BleState::Disconnected
    }

    async fn handle_command(&self, cmd: BleCommand) -> Result<BleResponse, Self::Error> {
        match cmd {
            BleCommand::GetConfig { section } => {
                match section {
                    ConfigSection::Network => {
                        let config = self.config.get_network_config().await;
                        // 简化：返回占位数据
                        Ok(BleResponse {
                            success: true,
                            message: "Network config retrieved".to_string(),
                            data: Some("{\"ssid\":\"configured\"}".to_string()),
                        })
                    }
                    ConfigSection::Time => {
                        Ok(BleResponse {
                            success: true,
                            message: "Time config retrieved".to_string(),
                            data: None,
                        })
                    }
                    ConfigSection::Display => {
                        Ok(BleResponse {
                            success: true,
                            message: "Display config retrieved".to_string(),
                            data: None,
                        })
                    }
                    ConfigSection::Power => {
                        Ok(BleResponse {
                            success: true,
                            message: "Power config retrieved".to_string(),
                            data: None,
                        })
                    }
                    ConfigSection::Log => {
                        Ok(BleResponse {
                            success: true,
                            message: "Log config retrieved".to_string(),
                            data: None,
                        })
                    }
                }
            }

            BleCommand::SetConfig { section, data } => {
                log::info!("Config updated for {:?}: {}", section, data);
                Ok(BleResponse {
                    success: true,
                    message: "Config updated".to_string(),
                    data: None,
                })
            }

            BleCommand::SyncTime { timestamp } => {
                log::info!("Time synced: {}", timestamp);
                Ok(BleResponse {
                    success: true,
                    message: "Time synced".to_string(),
                    data: None,
                })
            }

            BleCommand::SetWiFi { ssid, password } => {
                log::info!("WiFi credentials set for: {}", ssid);
                // 保存到配置
                Ok(BleResponse {
                    success: true,
                    message: "WiFi credentials saved".to_string(),
                    data: None,
                })
            }

            BleCommand::TestWiFi => {
                match self.wifi.test_connection().await {
                    result => Ok(BleResponse {
                        success: result.success,
                        message: result.message,
                        data: Some(serde_json::to_string(&result).unwrap_or_default()),
                    }),
                }
            }

            BleCommand::RefreshWeather => {
                log::info!("Weather refresh requested");
                Ok(BleResponse {
                    success: true,
                    message: "Weather refresh started".to_string(),
                    data: None,
                })
            }

            BleCommand::ForceRefresh => {
                log::info!("Force refresh requested");
                Ok(BleResponse {
                    success: true,
                    message: "Force refresh triggered".to_string(),
                    data: None,
                })
            }

            BleCommand::GetStatus => {
                Ok(BleResponse {
                    success: true,
                    message: "Status OK".to_string(),
                    data: None,
                })
            }

            BleCommand::StartOta => {
                log::info!("OTA started");
                Ok(BleResponse {
                    success: true,
                    message: "OTA started".to_string(),
                    data: None,
                })
            }

            BleCommand::OtaFirmware { chunk, offset } => {
                log::debug!("OTA chunk received at offset {}: {} bytes", offset, chunk.len());
                Ok(BleResponse {
                    success: true,
                    message: format!("Received {} bytes", chunk.len()),
                    data: None,
                })
            }

            BleCommand::FinishOta => {
                log::info!("OTA finished");
                Ok(BleResponse {
                    success: true,
                    message: "OTA finished".to_string(),
                    data: None,
                })
            }
        }
    }

    async fn poll_command(&self) -> Option<BleCommand> {
        let mut rx = self.cmd_rx.write().await;
        rx.try_recv().ok()
    }

    async fn push_response(&self, resp: BleResponse) -> Result<(), Self::Error> {
        log::debug!("BLE response: {:?}", resp);
        Ok(())
    }
}
