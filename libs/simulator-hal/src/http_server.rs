//! HTTP Control Server for Simulator

use tiny_http::{Server, Request, Response, Header, Method, StatusCode};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::{SimulatorConfig, SimulatedBle, SimulatedWifi, SimulatorState};
use lxx_calendar_common::traits::ble::{BleCommand, ConfigSection};

/// HTTP Response structure
#[derive(Debug, Serialize)]
pub struct HttpResponse {
    pub success: bool,
    pub message: String,
    pub data: Option<serde_json::Value>,
}

/// HTTP Control Server
pub struct HttpServer {
    state: Arc<SimulatorState>,
}

impl HttpServer {
    pub fn new(state: Arc<SimulatorState>) -> Self {
        Self { state }
    }

    /// Start the HTTP server
    pub fn run(&self, port: u16) {
        let server = match Server::http(format!("0.0.0.0:{}", port)) {
            Ok(s) => {
                log::info!("HTTP control server listening on port {}", port);
                s
            }
            Err(e) => {
                log::error!("Failed to start HTTP server on port {}: {}", port, e);
                return;
            }
        };

        for request in server.incoming_requests() {
            self.handle_request(request);
        }
    }

    fn handle_request(&self, request: Request) {
        let url = request.url().to_string();
        let method = request.method().clone();

        log::debug!("HTTP request: {} {}", method, url);

        let response = match (method, url.as_str()) {
            // ===== Config Endpoints =====
            (Method::Get, "/config") => self.get_all_config(),
            (Method::Get, "/config/network") => self.get_network_config(),
            (Method::Get, "/config/time") => self.get_time_config(),
            (Method::Get, "/config/display") => self.get_display_config(),
            (Method::Get, "/config/power") => self.get_power_config(),
            (Method::Get, "/config/log") => self.get_log_config(),
            (Method::Post, "/config/network") => self.set_network_config(request),
            (Method::Post, "/config/time") => self.set_time_config(request),
            (Method::Post, "/config/display") => self.set_display_config(request),
            (Method::Post, "/config/power") => self.set_power_config(request),
            (Method::Post, "/config/log") => self.set_log_config(request),

            // ===== WiFi Endpoints =====
            (Method::Get, "/wifi/scan") => self.wifi_scan(),
            (Method::Post, "/wifi/connect") => self.wifi_connect(request),
            (Method::Post, "/wifi/disconnect") => self.wifi_disconnect(),
            (Method::Post, "/wifi/test") => self.wifi_test(),
            (Method::Get, "/wifi/status") => self.wifi_status(),

            // ===== BLE Endpoints =====
            (Method::Post, "/ble/connect") => self.ble_connect(),
            (Method::Post, "/ble/disconnect") => self.ble_disconnect(),
            (Method::Post, "/ble/sync_time") => self.ble_sync_time(request),
            (Method::Post, "/ble/refresh_weather") => self.ble_refresh_weather(),
            (Method::Post, "/ble/send") => self.ble_send(request),

            // ===== Status =====
            (Method::Get, "/status") => self.get_status(),

            _ => HttpResponse {
                success: false,
                message: "Not found".to_string(),
                data: None,
            },
        };

        let status_code = if response.success {
            StatusCode(200)
        } else {
            StatusCode(400)
        };

        let http_response = Response::from_string(
            serde_json::to_string(&response).unwrap_or_else(|_| r#"{"success":false,"message":"Serialization error"}"#.to_string())
        )
        .with_status_code(status_code)
        .with_header(Header::build("Content-Type", "application/json"));

        let _ = request.respond(http_response);
    }

    // ===== Config Handlers =====
    fn get_all_config(&self) -> HttpResponse {
        // TODO: Serialize full config
        HttpResponse {
            success: true,
            message: "Config retrieved".to_string(),
            data: None,
        }
    }

    fn get_network_config(&self) -> HttpResponse {
        // TODO: Serialize network config
        HttpResponse {
            success: true,
            message: "Network config retrieved".to_string(),
            data: None,
        }
    }

    fn get_time_config(&self) -> HttpResponse {
        HttpResponse {
            success: true,
            message: "Time config retrieved".to_string(),
            data: None,
        }
    }

    fn get_display_config(&self) -> HttpResponse {
        HttpResponse {
            success: true,
            message: "Display config retrieved".to_string(),
            data: None,
        }
    }

    fn get_power_config(&self) -> HttpResponse {
        HttpResponse {
            success: true,
            message: "Power config retrieved".to_string(),
            data: None,
        }
    }

    fn get_log_config(&self) -> HttpResponse {
        HttpResponse {
            success: true,
            message: "Log config retrieved".to_string(),
            data: None,
        }
    }

    fn set_network_config(&self, request: Request) -> HttpResponse {
        // TODO: Parse and update network config
        HttpResponse {
            success: true,
            message: "Network config updated".to_string(),
            data: None,
        }
    }

    fn set_time_config(&self, request: Request) -> HttpResponse {
        HttpResponse {
            success: true,
            message: "Time config updated".to_string(),
            data: None,
        }
    }

    fn set_display_config(&self, request: Request) -> HttpResponse {
        HttpResponse {
            success: true,
            message: "Display config updated".to_string(),
            data: None,
        }
    }

    fn set_power_config(&self, request: Request) -> HttpResponse {
        HttpResponse {
            success: true,
            message: "Power config updated".to_string(),
            data: None,
        }
    }

    fn set_log_config(&self, request: Request) -> HttpResponse {
        HttpResponse {
            success: true,
            message: "Log config updated".to_string(),
            data: None,
        }
    }

    // ===== WiFi Handlers =====
    fn wifi_scan(&self) -> HttpResponse {
        // TODO: Return scanned networks
        HttpResponse {
            success: true,
            message: "WiFi scan completed".to_string(),
            data: Some(serde_json::json!([
                {"ssid": "Home-WiFi", "rssi": -50, "encrypted": true},
                {"ssid": "Office-Guest", "rssi": -70, "encrypted": true}
            ])),
        }
    }

    fn wifi_connect(&self, request: Request) -> HttpResponse {
        // TODO: Parse SSID/password and connect
        HttpResponse {
            success: true,
            message: "WiFi connect initiated".to_string(),
            data: None,
        }
    }

    fn wifi_disconnect(&self) -> HttpResponse {
        HttpResponse {
            success: true,
            message: "WiFi disconnected".to_string(),
            data: None,
        }
    }

    fn wifi_test(&self) -> HttpResponse {
        // TODO: Actually test connection
        HttpResponse {
            success: true,
            message: "WiFi test completed".to_string(),
            data: None,
        }
    }

    fn wifi_status(&self) -> HttpResponse {
        HttpResponse {
            success: true,
            message: "WiFi status retrieved".to_string(),
            data: None,
        }
    }

    // ===== BLE Handlers =====
    fn ble_connect(&self) -> HttpResponse {
        HttpResponse {
            success: true,
            message: "BLE connected".to_string(),
            data: None,
        }
    }

    fn ble_disconnect(&self) -> HttpResponse {
        HttpResponse {
            success: true,
            message: "BLE disconnected".to_string(),
            data: None,
        }
    }

    fn ble_sync_time(&self, request: Request) -> HttpResponse {
        // TODO: Parse timestamp
        HttpResponse {
            success: true,
            message: "Time sync command sent".to_string(),
            data: None,
        }
    }

    fn ble_refresh_weather(&self) -> HttpResponse {
        HttpResponse {
            success: true,
            message: "Weather refresh command sent".to_string(),
            data: None,
        }
    }

    fn ble_send(&self, request: Request) -> HttpResponse {
        // TODO: Parse and send BLE command
        HttpResponse {
            success: true,
            message: "Command sent".to_string(),
            data: None,
        }
    }

    // ===== Status Handler =====
    fn get_status(&self) -> HttpResponse {
        HttpResponse {
            success: true,
            message: "running".to_string(),
            data: Some(serde_json::json!({
                "server": "ok",
                "config": "ok",
                "wifi": "ok",
                "ble": "ok",
            })),
        }
    }
}
