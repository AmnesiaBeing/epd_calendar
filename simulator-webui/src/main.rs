use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, Json},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::services::ServeDir;

#[derive(Clone)]
struct AppState {
    simulator_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct StatusResponse {
    rtc: RtcStatus,
    ble: BleStatus,
    watchdog: WatchdogStatus,
}

#[derive(Debug, Serialize, Deserialize)]
struct RtcStatus {
    timestamp: i64,
    initialized: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct BleStatus {
    connected: bool,
    advertising: bool,
    configured: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct WatchdogStatus {
    enabled: bool,
    timeout_ms: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct ButtonRequest {
    event: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ApiResponse {
    success: bool,
    message: String,
}

async fn get_status(State(state): State<Arc<AppState>>) -> Result<Json<StatusResponse>, StatusCode> {
    let url = format!("{}/status", state.simulator_url);
    let client = reqwest::Client::new();
    
    match client.get(&url).send().await {
        Ok(resp) => {
            if let Ok(status) = resp.json::<StatusResponse>().await {
                Ok(Json(status))
            } else {
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
        Err(_) => Err(StatusCode::BAD_GATEWAY),
    }
}

async fn press_button(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ButtonRequest>,
) -> Result<Json<ApiResponse>, StatusCode> {
    let url = format!("{}/api/button", state.simulator_url);
    let client = reqwest::Client::new();
    
    match client.post(&url).json(&req).send().await {
        Ok(resp) => {
            if let Ok(api_resp) = resp.json::<ApiResponse>().await {
                Ok(Json(api_resp))
            } else {
                Ok(Json(ApiResponse { success: false, message: "Failed to parse response".to_string() }))
            }
        }
        Err(e) => Ok(Json(ApiResponse { success: false, message: e.to_string() })),
    }
}

async fn ble_connect(State(state): State<Arc<AppState>>) -> Result<Json<ApiResponse>, StatusCode> {
    let url = format!("{}/api/ble/connect", state.simulator_url);
    let client = reqwest::Client::new();
    
    match client.post(&url).send().await {
        Ok(resp) => {
            if let Ok(api_resp) = resp.json::<ApiResponse>().await {
                Ok(Json(api_resp))
            } else {
                Ok(Json(ApiResponse { success: false, message: "Failed to parse response".to_string() }))
            }
        }
        Err(e) => Ok(Json(ApiResponse { success: false, message: e.to_string() })),
    }
}

async fn ble_disconnect(State(state): State<Arc<AppState>>) -> Result<Json<ApiResponse>, StatusCode> {
    let url = format!("{}/api/ble/disconnect", state.simulator_url);
    let client = reqwest::Client::new();
    
    match client.post(&url).send().await {
        Ok(resp) => {
            if let Ok(api_resp) = resp.json::<ApiResponse>().await {
                Ok(Json(api_resp))
            } else {
                Ok(Json(ApiResponse { success: false, message: "Failed to parse response".to_string() }))
            }
        }
        Err(e) => Ok(Json(ApiResponse { success: false, message: e.to_string() })),
    }
}

async fn ble_config(State(state): State<Arc<AppState>>, Json(data): Json<serde_json::Value>) -> Result<Json<ApiResponse>, StatusCode> {
    let url = format!("{}/api/ble/config", state.simulator_url);
    let client = reqwest::Client::new();
    
    #[derive(Serialize)]
    struct ConfigRequest {
        data: serde_json::Value,
    }
    
    match client.post(&url).json(&ConfigRequest { data }).send().await {
        Ok(resp) => {
            if let Ok(api_resp) = resp.json::<ApiResponse>().await {
                Ok(Json(api_resp))
            } else {
                Ok(Json(ApiResponse { success: false, message: "Failed to parse response".to_string() }))
            }
        }
        Err(e) => Ok(Json(ApiResponse { success: false, message: e.to_string() })),
    }
}

fn get_html() -> Html<String> {
    Html(r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Simulator 控制面板</title>
    <style>
        * { box-sizing: border-box; margin: 0; padding: 0; }
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            min-height: 100vh;
            padding: 20px;
        }
        .container {
            max-width: 800px;
            margin: 0 auto;
        }
        h1 {
            color: white;
            text-align: center;
            margin-bottom: 30px;
            font-size: 2rem;
        }
        .card {
            background: white;
            border-radius: 12px;
            padding: 20px;
            margin-bottom: 20px;
            box-shadow: 0 4px 6px rgba(0,0,0,0.1);
        }
        .card h2 {
            color: #333;
            margin-bottom: 15px;
            font-size: 1.2rem;
        }
        .status-grid {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
            gap: 15px;
        }
        .status-item {
            background: #f5f5f5;
            padding: 15px;
            border-radius: 8px;
        }
        .status-item .label {
            color: #666;
            font-size: 0.9rem;
            margin-bottom: 5px;
        }
        .status-item .value {
            color: #333;
            font-size: 1.1rem;
            font-weight: bold;
        }
        .status-item .value.on { color: #22c55e; }
        .status-item .value.off { color: #ef4444; }
        .button-grid {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(120px, 1fr));
            gap: 10px;
        }
        button {
            padding: 12px 20px;
            border: none;
            border-radius: 8px;
            font-size: 1rem;
            cursor: pointer;
            transition: all 0.2s;
            font-weight: 500;
        }
        button:hover {
            transform: translateY(-2px);
            box-shadow: 0 4px 12px rgba(0,0,0,0.15);
        }
        button:active {
            transform: translateY(0);
        }
        .btn-primary { background: #3b82f6; color: white; }
        .btn-success { background: #22c55e; color: white; }
        .btn-danger { background: #ef4444; color: white; }
        .btn-secondary { background: #6b7280; color: white; }
        .btn-warning { background: #f59e0b; color: white; }
        .config-form textarea {
            width: 100%;
            height: 150px;
            padding: 10px;
            border: 1px solid #ddd;
            border-radius: 8px;
            font-family: monospace;
            font-size: 0.9rem;
            resize: vertical;
        }
        .config-form button {
            margin-top: 10px;
        }
        .toast {
            position: fixed;
            top: 20px;
            right: 20px;
            padding: 15px 20px;
            border-radius: 8px;
            color: white;
            font-weight: 500;
            transform: translateX(400px);
            transition: transform 0.3s ease;
            z-index: 1000;
        }
        .toast.show { transform: translateX(0); }
        .toast.success { background: #22c55e; }
        .toast.error { background: #ef4444; }
    </style>
</head>
<body>
    <div class="container">
        <h1>Simulator 控制面板</h1>
        
        <div class="card">
            <h2>设备状态</h2>
            <div class="status-grid">
                <div class="status-item">
                    <div class="label">RTC 时间戳</div>
                    <div class="value" id="rtc-timestamp">-</div>
                </div>
                <div class="status-item">
                    <div class="label">RTC 已初始化</div>
                    <div class="value" id="rtc-initialized">-</div>
                </div>
                <div class="status-item">
                    <div class="label">BLE 连接状态</div>
                    <div class="value" id="ble-connected">-</div>
                </div>
                <div class="status-item">
                    <div class="label">BLE 广播状态</div>
                    <div class="value" id="ble-advertising">-</div>
                </div>
                <div class="status-item">
                    <div class="label">BLE 已配置</div>
                    <div class="value" id="ble-configured">-</div>
                </div>
                <div class="status-item">
                    <div class="label">看门狗状态</div>
                    <div class="value" id="watchdog-enabled">-</div>
                </div>
                <div class="status-item">
                    <div class="label">看门狗超时</div>
                    <div class="value" id="watchdog-timeout">-</div>
                </div>
            </div>
            <button class="btn-secondary" onclick="refreshStatus()" style="margin-top: 15px;">刷新状态</button>
        </div>
        
        <div class="card">
            <h2>按键模拟</h2>
            <div class="button-grid">
                <button class="btn-primary" onclick="pressButton('short_press')">短按</button>
                <button class="btn-warning" onclick="pressButton('long_press')">长按</button>
                <button class="btn-success" onclick="pressButton('double_click')">双击</button>
                <button class="btn-danger" onclick="pressButton('triple_click')">三击</button>
            </div>
        </div>
        
        <div class="card">
            <h2>BLE 模拟</h2>
            <div class="button-grid">
                <button class="btn-success" onclick="bleConnect()">连接</button>
                <button class="btn-danger" onclick="bleDisconnect()">断开</button>
            </div>
        </div>
        
        <div class="card">
            <h2>BLE 配置</h2>
            <div class="config-form">
                <textarea id="config-data" placeholder="输入配置 JSON...">
{
  "version": 1,
  "network_config": {
    "wifi_ssid": "MyWiFi",
    "wifi_password": "password123",
    "weather_api_key": "api_key_here",
    "location_id": "101010100",
    "sync_interval_minutes": 60
  }
}</textarea>
                <button class="btn-primary" onclick="applyConfig()">应用配置</button>
            </div>
        </div>
    </div>
    
    <div id="toast" class="toast"></div>
    
    <script>
        const simulatorUrl = 'http://' + window.location.hostname + ':8080';
        
        function showToast(message, type = 'success') {
            const toast = document.getElementById('toast');
            toast.textContent = message;
            toast.className = 'toast ' + type + ' show';
            setTimeout(() => {
                toast.className = 'toast';
            }, 3000);
        }
        
        async function refreshStatus() {
            try {
                const resp = await fetch(simulatorUrl + '/status');
                const data = await resp.json();
                
                document.getElementById('rtc-timestamp').textContent = data.rtc.timestamp;
                document.getElementById('rtc-initialized').textContent = data.rtc.initialized ? '是' : '否';
                
                const bleConnected = document.getElementById('ble-connected');
                bleConnected.textContent = data.ble.connected ? '已连接' : '未连接';
                bleConnected.className = 'value ' + (data.ble.connected ? 'on' : 'off');
                
                const bleAdvertising = document.getElementById('ble-advertising');
                bleAdvertising.textContent = data.ble.advertising ? '广播中' : '未广播';
                bleAdvertising.className = 'value ' + (data.ble.advertising ? 'on' : 'off');
                
                const bleConfigured = document.getElementById('ble-configured');
                bleConfigured.textContent = data.ble.configured ? '已配置' : '未配置';
                bleConfigured.className = 'value ' + (data.ble.configured ? 'on' : 'off');
                
                const watchdogEnabled = document.getElementById('watchdog-enabled');
                watchdogEnabled.textContent = data.watchdog.enabled ? '启用' : '禁用';
                watchdogEnabled.className = 'value ' + (data.watchdog.enabled ? 'on' : 'off');
                
                document.getElementById('watchdog-timeout').textContent = data.watchdog.timeout_ms + 'ms';
                
                showToast('状态已刷新');
            } catch (e) {
                showToast('获取状态失败: ' + e.message, 'error');
            }
        }
        
        async function pressButton(event) {
            try {
                const resp = await fetch(simulatorUrl + '/api/button', {
                    method: 'POST',
                    headers: {'Content-Type': 'application/json'},
                    body: JSON.stringify({event})
                });
                const data = await resp.json();
                showToast(data.message);
                refreshStatus();
            } catch (e) {
                showToast('操作失败: ' + e.message, 'error');
            }
        }
        
        async function bleConnect() {
            try {
                const resp = await fetch(simulatorUrl + '/api/ble/connect', {method: 'POST'});
                const data = await resp.json();
                showToast(data.message);
                refreshStatus();
            } catch (e) {
                showToast('操作失败: ' + e.message, 'error');
            }
        }
        
        async function bleDisconnect() {
            try {
                const resp = await fetch(simulatorUrl + '/api/ble/disconnect', {method: 'POST'});
                const data = await resp.json();
                showToast(data.message);
                refreshStatus();
            } catch (e) {
                showToast('操作失败: ' + e.message, 'error');
            }
        }
        
        async function applyConfig() {
            try {
                const configData = JSON.parse(document.getElementById('config-data').value);
                const resp = await fetch(simulatorUrl + '/api/ble/config', {
                    method: 'POST',
                    headers: {'Content-Type': 'application/json'},
                    body: JSON.stringify({data: configData})
                });
                const data = await resp.json();
                showToast(data.message);
                refreshStatus();
            } catch (e) {
                showToast('配置失败: ' + e.message, 'error');
            }
        }
        
        refreshStatus();
        setInterval(refreshStatus, 5000);
    </script>
</body>
</html>"#.to_string())
}

async fn webui() -> Html<String> {
    get_html()
}

#[tokio::main]
async fn main() {
    let simulator_url = std::env::var("SIMULATOR_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:8080".to_string());
    
    let port: u16 = std::env::var("WEBUI_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8081);
    
    tracing_subscriber::fmt()
        .with_env_filter("simulator_webui=debug,tower_http=debug")
        .init();
    
    let state = Arc::new(AppState { simulator_url });
    
    let app = Router::new()
        .route("/", get(webui))
        .route("/api/status", get(get_status))
        .route("/api/button", post(press_button))
        .route("/api/ble/connect", post(ble_connect))
        .route("/api/ble/disconnect", post(ble_disconnect))
        .route("/api/ble/config", post(ble_config))
        .with_state(state);
    
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    
    tracing::info!("WebUI server starting on http://0.0.0.0:{}", port);
    
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
