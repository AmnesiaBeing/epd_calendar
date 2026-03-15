use std::io::Read;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use tiny_http::{Response, Server};

use crate::SimulatorControl;
use crate::ble::SimulatedBLE;
use crate::button::SimulatorButton;
use crate::control::types::*;
use lxx_calendar_common::traits::button::ButtonEvent;
use lxx_calendar_common::{debug, error, info, warn};

pub struct HttpServer {
    control: Arc<Mutex<SimulatorControl>>,
    ble: Arc<Mutex<SimulatedBLE>>,
    button: Arc<Mutex<SimulatorButton>>,
    port: u16,
}

impl HttpServer {
    pub fn new(
        control: Arc<Mutex<SimulatorControl>>,
        ble: Arc<Mutex<SimulatedBLE>>,
        button: Arc<Mutex<SimulatorButton>>,
        port: u16,
    ) -> Self {
        Self {
            control,
            ble,
            button,
            port,
        }
    }

    pub fn run(&self) {
        let port = self.find_available_port();
        let addr = format!("127.0.0.1:{}", port);

        let server = match Server::http(&addr) {
            Ok(s) => s,
            Err(e) => {
                error!("Failed to start HTTP server on {}: {}", addr, e);
                return;
            }
        };

        info!("Simulator HTTP server started on http://{}", addr);

        for mut request in server.incoming_requests() {
            let control = Arc::clone(&self.control);
            let ble = Arc::clone(&self.ble);
            let button = Arc::clone(&self.button);
            let method = request.method().as_str().to_string();
            let url = request.url().to_string();
            let body = read_body(request.as_reader());

            let response = handle_request(control, ble, button, &method, &url, &body);
            request.respond(response).ok();
        }
    }

    fn find_available_port(&self) -> u16 {
        let mut port = self.port;
        for _ in 0..100 {
            let addr = format!("127.0.0.1:{}", port);
            if std::net::TcpListener::bind(&addr).is_ok() {
                return port;
            }
            port += 1;
        }
        warn!(
            "Could not find available port near {}, using default",
            self.port
        );
        self.port
    }
}

fn handle_request(
    control: Arc<Mutex<SimulatorControl>>,
    ble: Arc<Mutex<SimulatedBLE>>,
    button: Arc<Mutex<SimulatorButton>>,
    method: &str,
    url: &str,
    body: &str,
) -> Response<std::io::Cursor<Vec<u8>>> {
    debug!("HTTP {} {}", method, url);

    match (method, url) {
        // 基础状态端点
        ("GET", "/status") => handle_get_status(control, ble, button),
        ("GET", "/status/rtc") => handle_get_rtc_status(control, ble, button),
        ("GET", "/status/ble") => handle_get_ble_status(control, ble, button),
        ("GET", "/status/watchdog") => handle_get_watchdog_status(control, ble, button),

        // 显示相关端点
        ("GET", "/status/display") => handle_get_display_status(control),
        ("POST", "/api/display/refresh") => handle_display_refresh(control, body),
        ("POST", "/api/display/mode") => handle_display_mode(control, body),

        // 闹钟相关端点
        ("GET", "/status/alarm") => handle_get_alarm_status(control),
        ("POST", "/api/alarm") => handle_alarm(control, body),

        // BLE 和按键端点
        ("POST", "/api/button") => handle_button(control, ble, button, body),
        ("POST", "/api/ble/connect") => handle_ble_connect(control, ble, button),
        ("POST", "/api/ble/disconnect") => handle_ble_disconnect(control, ble, button),
        ("POST", "/api/ble/config") => handle_ble_config(control, ble, button, body),

        _ => not_found(),
    }
}

fn handle_get_status(
    control: Arc<Mutex<SimulatorControl>>,
    ble: Arc<Mutex<SimulatedBLE>>,
    _button: Arc<Mutex<SimulatorButton>>,
) -> Response<std::io::Cursor<Vec<u8>>> {
    let ctrl = control.lock().unwrap();
    let b = ble.lock().unwrap();
    let status = ctrl.get_status(&b);
    json_response(&status)
}

fn handle_get_rtc_status(
    control: Arc<Mutex<SimulatorControl>>,
    ble: Arc<Mutex<SimulatedBLE>>,
    _button: Arc<Mutex<SimulatorButton>>,
) -> Response<std::io::Cursor<Vec<u8>>> {
    let ctrl = control.lock().unwrap();
    let status = ctrl.get_rtc_status();
    json_response(&status)
}

fn handle_get_ble_status(
    control: Arc<Mutex<SimulatorControl>>,
    ble: Arc<Mutex<SimulatedBLE>>,
    _button: Arc<Mutex<SimulatorButton>>,
) -> Response<std::io::Cursor<Vec<u8>>> {
    let ctrl = control.lock().unwrap();
    let b = ble.lock().unwrap();
    let status = ctrl.get_ble_status(&b);
    json_response(&status)
}

fn handle_get_watchdog_status(
    control: Arc<Mutex<SimulatorControl>>,
    ble: Arc<Mutex<SimulatedBLE>>,
    _button: Arc<Mutex<SimulatorButton>>,
) -> Response<std::io::Cursor<Vec<u8>>> {
    let ctrl = control.lock().unwrap();
    let status = ctrl.get_watchdog_status();
    json_response(&status)
}

fn handle_button(
    _control: Arc<Mutex<SimulatorControl>>,
    _ble: Arc<Mutex<SimulatedBLE>>,
    button: Arc<Mutex<SimulatorButton>>,
    body: &str,
) -> Response<std::io::Cursor<Vec<u8>>> {
    match serde_json::from_str::<ButtonRequest>(body) {
        Ok(req) => {
            let event = req.event.clone();
            {
                let btn = button.lock().unwrap();
                let btn_event = match req.event {
                    ButtonEventType::ShortPress => ButtonEvent::ShortPress,
                    ButtonEventType::LongPress => ButtonEvent::LongPress,
                    ButtonEventType::DoubleClick => ButtonEvent::DoubleClick,
                    ButtonEventType::TripleClick => ButtonEvent::TripleClick,
                };
                btn.simulate_press(btn_event);
            }
            let resp = ButtonResponse {
                success: true,
                message: format!("Button {:?} simulated", event),
            };
            json_response(&resp)
        }
        Err(e) => bad_request(&format!("Invalid request: {}", e)),
    }
}

fn handle_ble_connect(
    _control: Arc<Mutex<SimulatorControl>>,
    ble: Arc<Mutex<SimulatedBLE>>,
    _button: Arc<Mutex<SimulatorButton>>,
) -> Response<std::io::Cursor<Vec<u8>>> {
    {
        let mut b = ble.lock().unwrap();
        b.simulate_connect();
    }
    let resp = BleConnectResponse {
        success: true,
        message: "BLE connected".to_string(),
    };
    json_response(&resp)
}

fn handle_ble_disconnect(
    _control: Arc<Mutex<SimulatorControl>>,
    ble: Arc<Mutex<SimulatedBLE>>,
    _button: Arc<Mutex<SimulatorButton>>,
) -> Response<std::io::Cursor<Vec<u8>>> {
    {
        let mut b = ble.lock().unwrap();
        b.simulate_disconnect();
    }
    let resp = BleConnectResponse {
        success: true,
        message: "BLE disconnected".to_string(),
    };
    json_response(&resp)
}

fn handle_ble_config(
    control: Arc<Mutex<SimulatorControl>>,
    ble: Arc<Mutex<SimulatedBLE>>,
    _button: Arc<Mutex<SimulatorButton>>,
    body: &str,
) -> Response<std::io::Cursor<Vec<u8>>> {
    match serde_json::from_str::<BleConfigRequest>(body) {
        Ok(_req) => {
            // 将整个请求体作为数据传递
            let data = body.as_bytes();

            // 设置 BLE configured 状态
            {
                let mut b = ble.lock().unwrap();
                // 调用 simulate_config 来设置状态和触发数据回调
                b.simulate_config(data);
            }

            // 触发 SimulatorControl 的 BLE 配置回调，将配置传递给 StateManager
            {
                let ctrl = control.lock().unwrap();
                ctrl.simulate_ble_config(data);
            }

            let resp = BleConfigResponse {
                success: true,
                change: Some("Config received and being processed".to_string()),
                message: "Config applied".to_string(),
            };
            json_response(&resp)
        }
        Err(e) => bad_request(&format!("Invalid request: {}", e)),
    }
}

// ==================== 显示相关处理函数 ====================

fn handle_get_display_status(
    control: Arc<Mutex<SimulatorControl>>,
) -> Response<std::io::Cursor<Vec<u8>>> {
    let ctrl = control.lock().unwrap();
    let status = ctrl.get_display_status();
    json_response(&status)
}

fn handle_display_refresh(
    control: Arc<Mutex<SimulatorControl>>,
    body: &str,
) -> Response<std::io::Cursor<Vec<u8>>> {
    match serde_json::from_str::<DisplayRefreshRequest>(body) {
        Ok(req) => {
            let ctrl = control.lock().unwrap();
            ctrl.refresh_display(&req.mode);

            let resp = DisplayRefreshResponse {
                success: true,
                message: format!("Display refreshed in {} mode", req.mode),
            };
            json_response(&resp)
        }
        Err(e) => bad_request(&format!("Invalid request: {}", e)),
    }
}

fn handle_display_mode(
    control: Arc<Mutex<SimulatorControl>>,
    body: &str,
) -> Response<std::io::Cursor<Vec<u8>>> {
    match serde_json::from_str::<DisplayModeRequest>(body) {
        Ok(req) => {
            let valid_modes = ["normal", "power_save", "high_quality"];
            if !valid_modes.contains(&req.mode.as_str()) {
                return bad_request(&format!(
                    "Invalid mode: {}. Valid modes: {:?}",
                    req.mode, valid_modes
                ));
            }

            let ctrl = control.lock().unwrap();
            ctrl.set_display_mode(&req.mode);

            let resp = DisplayModeResponse {
                success: true,
                message: format!("Display mode set to {}", req.mode),
            };
            json_response(&resp)
        }
        Err(e) => bad_request(&format!("Invalid request: {}", e)),
    }
}

// ==================== 闹钟相关处理函数 ====================

fn handle_get_alarm_status(
    control: Arc<Mutex<SimulatorControl>>,
) -> Response<std::io::Cursor<Vec<u8>>> {
    let ctrl = control.lock().unwrap();
    let status = ctrl.get_alarm_status();
    json_response(&status)
}

fn handle_alarm(
    control: Arc<Mutex<SimulatorControl>>,
    body: &str,
) -> Response<std::io::Cursor<Vec<u8>>> {
    match serde_json::from_str::<AlarmRequest>(body) {
        Ok(req) => {
            let ctrl = control.lock().unwrap();

            match req.action.as_str() {
                "set" => {
                    let trigger_time = if let Some(delay) = req.delay_seconds {
                        let rtc = ctrl.rtc.lock().unwrap();
                        Some(rtc.get_timestamp() as u64 + delay)
                    } else if let Some(_time_str) = req.trigger_time {
                        // 简化处理：使用当前时间 + 5 秒
                        let now = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap()
                            .as_secs();
                        Some(now + 5)
                    } else {
                        None
                    };

                    ctrl.set_alarm(trigger_time);

                    let resp = AlarmResponse {
                        success: true,
                        message: "Alarm set successfully".to_string(),
                    };
                    json_response(&resp)
                }
                "cancel" => {
                    ctrl.cancel_alarm();

                    let resp = AlarmResponse {
                        success: true,
                        message: "Alarm cancelled".to_string(),
                    };
                    json_response(&resp)
                }
                "trigger" => {
                    // 用于测试手动触发
                    ctrl.trigger_alarm();

                    let resp = AlarmResponse {
                        success: true,
                        message: "Alarm triggered manually".to_string(),
                    };
                    json_response(&resp)
                }
                _ => bad_request(&format!("Unknown alarm action: {}", req.action)),
            }
        }
        Err(e) => bad_request(&format!("Invalid request: {}", e)),
    }
}

fn not_found() -> Response<std::io::Cursor<Vec<u8>>> {
    let resp = serde_json::json!({"error": "Not found"});
    Response::from_string(resp.to_string())
        .with_status_code(404)
        .with_header(
            tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap(),
        )
        .into()
}

fn bad_request(msg: &str) -> Response<std::io::Cursor<Vec<u8>>> {
    let resp = serde_json::json!({"error": msg});
    Response::from_string(resp.to_string())
        .with_status_code(400)
        .with_header(
            tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap(),
        )
        .into()
}

fn read_body<R: Read>(mut reader: R) -> String {
    let mut body = String::new();
    reader.read_to_string(&mut body).ok();
    body
}

fn json_response<T: serde::Serialize>(value: &T) -> Response<std::io::Cursor<Vec<u8>>> {
    let json = serde_json::to_string(value).unwrap_or_else(|_| "{}".to_string());
    Response::from_string(json)
        .with_header(
            tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap(),
        )
        .into()
}
