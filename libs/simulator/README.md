# Simulator 调试接口文档

## 概述

Simulator 平台提供 HTTP API 用于外部控制模拟硬件行为，包括按钮按下、BLE 连接、BLE 配置等。

## 启动

```bash
# 默认端口 8080
RUST_LOG=debug cargo rs

# 或指定端口
SIMULATOR_PORT=8081 RUST_LOG=debug cargo rs
```

服务器绑定地址: `127.0.0.1` (仅本地访问)

---

## API 接口

### 1. 获取状态

获取 RTC、BLE、Watchdog 状态。

**请求**
```bash
GET /status
```

**响应**
```json
{
  "rtc": {
    "timestamp": 1771588453,
    "initialized": false
  },
  "ble": {
    "connected": false,
    "advertising": false,
    "configured": false
  },
  "watchdog": {
    "enabled": true,
    "timeout_ms": 30000
  }
}
```

---

 模拟按钮按下### 2.

模拟硬件按钮按下事件，可触发系统唤醒。

**请求**
```bash
POST /api/button
Content-Type: application/json

{
  "event": "short_press"  // short_press | long_press | double_click | triple_click
}
```

**响应**
```json
{
  "success": true,
  "message": "Button ShortPress simulated"
}
```

**示例**
```bash
curl -X POST http://127.0.0.1:8080/api/button \
  -H "Content-Type: application/json" \
  -d '{"event": "short_press"}'
```

---

### 3. 模拟 BLE 连接

模拟 BLE 设备连接。

**请求**
```bash
POST /api/ble/connect
```

**响应**
```json
{
  "success": true,
  "message": "BLE connected"
}
```

**示例**
```bash
curl -X POST http://127.0.0.1:8080/api/ble/connect
```

---

### 4. 模拟 BLE 断开

模拟 BLE 设备断开连接。

**请求**
```bash
POST /api/ble/disconnect
```

**响应**
```json
{
  "success": true,
  "message": "BLE disconnected"
}
```

**示例**
```bash
curl -X POST http://127.0.0.1:8080/api/ble/disconnect
```

---

### 5. 模拟 BLE 配置

通过 BLE 接收配置数据，模拟配置下发。

**请求**
```bash
POST /api/ble/config
Content-Type: application/json

{
  "data": {
    "wifi_ssid": "MyNetwork",
    "wifi_password": "password123",
    "location": "Shanghai",
    "auto_sync": true,
    "sync_interval_minutes": 60
  }
}
```

**响应**
```json
{
  "success": true,
  "change": "DisplayConfig",
  "message": "Config applied"
}
```

**完整示例**
```bash
curl -X POST http://127.0.0.1:8080/api/ble/config \
  -H "Content-Type: application/json" \
  -d '{
    "data": {
      "wifi_ssid": "MyNetwork",
      "wifi_password": "password123",
      "location": "Shanghai",
      "auto_sync": true
    }
  }'
```

---

## 调试场景

### 场景 1: 测试按钮事件

```bash
# 1. 启动模拟器
cargo run -p lxx-calendar-boards-simulator &

# 2. 模拟短按按钮
curl -X POST http://127.0.0.1:8080/api/button \
  -H "Content-Type: application/json" \
  -d '{"event": "short_press"}'

# 3. 模拟长按按钮
curl -X POST http://127.0.0.1:8080/api/button \
  -H "Content-Type: application/json" \
  -d '{"event": "long_press"}'

# 4. 查看状态
curl http://127.0.0.1:8080/status
```

### 场景 2: 测试 BLE 配网

```bash
# 1. 模拟 BLE 连接
curl -X POST http://127.0.0.1:8080/api/ble/connect

# 2. 下发 WiFi 配置
curl -X POST http://127.0.0.1:8080/api/ble/config \
  -H "Content-Type: application/json" \
  -d '{
    "data": {
      "wifi_ssid": "HomeWiFi",
      "wifi_password": "homepassword",
      "location": "Beijing"
    }
  }'

# 3. 断开 BLE
curl -X POST http://127.0.0.1:8080/api/ble/disconnect

# 4. 查看最终状态
curl http://127.0.0.1:8080/status
```

### 场景 3: 测试睡眠唤醒

```bash
# 模拟器进入 light sleep 后，通过按钮 API 唤醒系统

# 1. 模拟按钮按下（系统睡眠时按下会唤醒）
curl -X POST http://127.0.0.1:8080/api/button \
  -H "Content-Type: application/json" \
  -d '{"event": "short_press"}'

# 2. 查看系统是否被唤醒
curl http://127.0.0.1:8080/status
```

---

## 注意事项

1. **端口占用**: 如果端口 8080 被占用，会自动尝试 8081, 8082... 直到找到可用端口
2. **仅本地访问**: 服务器绑定 `127.0.0.1`，外部网络无法访问
3. **ButtonEvent 类型**:
   - `short_press` - 短按
   - `long_press` - 长按
   - `double_click` - 双击
   - `triple_click` - 三击

---

## 环境变量

| 变量名 | 默认值 | 说明 |
|--------|--------|------|
| `SIMULATOR_PORT` | `8080` | HTTP 服务器端口 |

```bash
SIMULATOR_PORT=9000 cargo run -p lxx-calendar-boards-simulator
```
