//! Simulated WiFi Driver

use lxx_calendar_common::traits::wifi::{WifiController, WifiMode, WifiConfig};
use tokio::sync::RwLock;
use std::sync::Arc;

/// WiFi 连接状态
#[derive(Debug, Clone)]
pub struct WifiState {
    pub connected: bool,
    pub ssid: Option<String>,
    pub ip: Option<String>,
    pub rssi: Option<i16>,
}

/// Simulated WiFi
pub struct SimulatedWifi {
    state: Arc<RwLock<WifiState>>,
}

impl SimulatedWifi {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            state: Arc::new(RwLock::new(WifiState {
                connected: false,
                ssid: None,
                ip: None,
                rssi: None,
            })),
        })
    }

    /// 模拟器专用：手动设置连接状态
    pub async fn set_connected(&self, connected: bool, ssid: &str, ip: &str) {
        let mut state = self.state.write().await;
        state.connected = connected;
        state.ssid = if connected { Some(ssid.to_string()) } else { None };
        state.ip = if connected { Some(ip.to_string()) } else { None };
        state.rssi = if connected { Some(-50) } else { None };
        log::info!("WiFi state updated: connected={}, ssid={}", connected, ssid);
    }

    /// 模拟器专用：获取当前状态
    pub async fn get_state(&self) -> WifiState {
        self.state.read().await.clone()
    }

    /// 模拟器专用：测试连接
    pub async fn test_connection(&self) -> WifiTestResult {
        let state = self.state.read().await;

        if !state.connected {
            return WifiTestResult {
                success: false,
                ip: None,
                dns_resolved: false,
                http_test: false,
                message: "Not connected to WiFi".to_string(),
            };
        }

        // 模拟器：实际测试 DNS 和 HTTP
        let dns_ok = test_dns().await;
        let http_ok = test_http().await;

        WifiTestResult {
            success: dns_ok && http_ok,
            ip: state.ip.clone(),
            dns_resolved: dns_ok,
            http_test: http_ok,
            message: if dns_ok && http_ok {
                "Connection test passed".to_string()
            } else {
                "Partial connectivity".to_string()
            },
        }
    }
}

impl Default for SimulatedWifi {
    fn default() -> Self {
        Self::new()
    }
}

/// WiFi 测试结果
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WifiTestResult {
    pub success: bool,
    pub ip: Option<String>,
    pub dns_resolved: bool,
    pub http_test: bool,
    pub message: String,
}

/// 测试 DNS 解析
async fn test_dns() -> bool {
    use tokio::net::lookup_host;
    lookup_host("httpbin.org:80").await.is_ok()
}

/// 测试 HTTP 请求
async fn test_http() -> bool {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpStream;

    match TcpStream::connect("httpbin.org:80").await {
        Ok(mut stream) => {
            let request = "GET /get HTTP/1.1\r\nHost: httpbin.org\r\nConnection: close\r\n\r\n";
            if stream.write_all(request.as_bytes()).await.is_ok() {
                let mut buf = [0u8; 256];
                match stream.read(&mut buf).await {
                    Ok(n) => n > 0,
                    Err(_) => false,
                }
            } else {
                false
            }
        }
        Err(_) => false,
    }
}

impl WifiController for SimulatedWifi {
    type Error = std::io::Error;

    async fn connect_sta(&mut self, ssid: &str, password: &str) -> Result<(), Self::Error> {
        log::info!("WiFi connecting to: {}", ssid);

        if ssid.is_empty() || password.is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Empty SSID or password",
            ));
        }

        // 模拟器：不实际连接，只记录配置
        // 实际连接由用户通过 HTTP API 触发
        log::info!("WiFi credentials saved for: {}", ssid);
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<(), Self::Error> {
        let mut state = self.state.write().await;
        state.connected = false;
        state.ssid = None;
        state.ip = None;
        state.rssi = None;
        log::info!("WiFi disconnected");
        Ok(())
    }

    fn is_connected(&self) -> bool {
        // 注意：这里需要异步安全的方式
        false
    }
}
