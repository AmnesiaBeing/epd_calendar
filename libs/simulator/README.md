# Simulator 调试接口文档

## 概述

Simulator 平台提供 HTTP API 用于外部控制模拟硬件行为，包括按钮按下、BLE 连接、BLE 配置等。

### 架构说明

Simulator 使用 **Deep Sleep 循环** 架构：
- **HTTP 服务器**：在独立 tokio 线程运行，始终保持运行
- **Embassy 执行器**：在 `tokio::task::spawn_blocking` 中运行
- **Deep Sleep**：任务完成后进入 Deep Sleep（等待指定时间），然后重启 embassy 执行器

```
┌─────────────────────────────────────────────────────────┐
│                    Tokio Runtime                        │
│  ┌───────────────────────────────────────────────────┐  │
│  │  HTTP Server Thread (独立运行，始终保持)           │  │
│  │  - 处理 BLE 配置                                     │  │
│  │  - 处理按钮事件                                     │  │
│  └───────────────────────────────────────────────────┘  │
│                                                          │
│  ┌───────────────────────────────────────────────────┐  │
│  │  Embassy Executor (spawn_blocking)                │  │
│  │  - main_task (业务逻辑)                           │  │
│  │  - Deep Sleep (等待 60 秒)                          │  │
│  └───────────────────────────────────────────────────┘  │
│                                                          │
│  循环：重启 Embassy Executor                             │
└─────────────────────────────────────────────────────────┘
```

---

## 启动

```bash
# 默认端口 8080
RUST_LOG=info cargo rs

# 或指定端口
SIMULATOR_PORT=8081 RUST_LOG=debug cargo rs

# 后台运行并记录日志
RUST_LOG=info cargo rs > simulator.log 2>&1 &
```

服务器绑定地址：`127.0.0.1` (仅本地访问)

---

## API 接口

### 1. 获取系统状态

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
    "initialized": true
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

### 2. 模拟按钮按下

模拟硬件按钮按下事件，可触发系统唤醒或进入配对模式。

**请求**
```bash
POST /api/button
Content-Type: application/json

{
  "event": "short_press"
}
```

**事件类型**
- `short_press` - 短按（唤醒系统）
- `long_press` - 长按（恢复出厂设置）
- `double_click` - 双击
- `triple_click` - 三击（进入配对模式）

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
    "wifi_password": "password123"
  }
}
```

**响应**
```json
{
  "success": true,
  "change": "NetworkConfig",
  "message": "Config applied"
}
```

**完整示例**
```bash
curl -X POST http://127.0.0.1:8080/api/ble/config \
  -H "Content-Type: application/json" \
  -d '{
    "data": {
      "wifi_ssid": "HomeWiFi",
      "wifi_password": "homepassword"
    }
  }'
```

---

## 调试场景

### 场景 1: 测试按钮事件

```bash
# 1. 启动模拟器
cargo rs > simulator.log 2>&1 &

# 2. 模拟短按按钮
curl -X POST http://127.0.0.1:8080/api/button \
  -H "Content-Type: application/json" \
  -d '{"event": "short_press"}'

# 3. 模拟长按按钮
curl -X POST http://127.0.0.1:8080/api/button \
  -H "Content-Type: application/json" \
  -d '{"event": "long_press"}'

# 4. 查看状态
curl http://127.0.0.1:8080/status | python3 -m json.tool
```

---

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
      "wifi_password": "homepassword"
    }
  }'

# 3. 断开 BLE
curl -X POST http://127.0.0.1:8080/api/ble/disconnect

# 4. 查看最终状态
curl http://127.0.0.1:8080/status | python3 -m json.tool
```

---

### 场景 3: 测试 Deep Sleep 循环

```bash
# 1. 启动模拟器（详细日志）
RUST_LOG=debug cargo rs > simulator.log 2>&1 &

# 2. 等待 Deep Sleep 周期（约 60 秒）
sleep 65

# 3. 查看日志，确认 Deep Sleep 循环
grep "Deep Sleep cycle" simulator.log

# 预期输出:
# === Simulator Deep Sleep cycle starting ===
# ... (任务执行)
# Entering deep sleep for 60 seconds
# === Simulator Deep Sleep cycle ended, restarting ===
```

---

### 场景 4: 测试配置持久化

```bash
# 1. 清理旧配置
rm -f /tmp/simulator_flash.bin

# 2. 启动模拟器（首次启动，加载默认配置）
cargo rs > simulator.log 2>&1 &
sleep 3

# 3. 通过 BLE 设置配置
curl -X POST http://127.0.0.1:8080/api/ble/connect
curl -X POST http://127.0.0.1:8080/api/ble/config \
  -H "Content-Type: application/json" \
  -d '{"data":{"wifi_ssid":"TestWiFi","wifi_password":"12345678"}}'

# 4. 验证配置已保存
ls -lh /tmp/simulator_flash.bin

# 5. 重启模拟器
pkill -f lxx-calendar-boards-simulator
sleep 2
cargo rs > simulator2.log 2>&1 &
sleep 3

# 6. 验证配置在重启后保留
grep "Config" simulator2.log
```

---

## 自动化测试脚本

### 快速测试所有功能

```bash
#!/bin/bash
# test_simulator.sh

set -e

PORT=${SIMULATOR_PORT:-8080}
BASE_URL="http://127.0.0.1:$PORT"

echo "=== Simulator 功能测试 ==="

# 1. 启动模拟器
echo "[1/6] 启动模拟器..."
pkill -f lxx-calendar-boards-simulator 2>/dev/null || true
sleep 1
RUST_LOG=info cargo rs > /tmp/simulator_test.log 2>&1 &
sleep 5

# 2. 检查服务状态
echo "[2/6] 检查服务状态..."
curl -s $BASE_URL/status | python3 -m json.tool

# 3. 测试按钮事件
echo "[3/6] 测试按钮事件..."
curl -s -X POST $BASE_URL/api/button \
  -H "Content-Type: application/json" \
  -d '{"event": "short_press"}' | python3 -m json.tool

# 4. 测试 BLE 连接
echo "[4/6] 测试 BLE 连接..."
curl -s -X POST $BASE_URL/api/ble/connect | python3 -m json.tool

# 5. 测试 BLE 配置
echo "[5/6] 测试 BLE 配置..."
curl -s -X POST $BASE_URL/api/ble/config \
  -H "Content-Type: application/json" \
  -d '{"data":{"wifi_ssid":"TestWiFi","wifi_password":"12345678"}}' | python3 -m json.tool

# 6. 测试 BLE 断开
echo "[6/6] 测试 BLE 断开..."
curl -s -X POST $BASE_URL/api/ble/disconnect | python3 -m json.tool

echo ""
echo "=== 测试完成 ==="
echo "日志文件：/tmp/simulator_test.log"
echo "清理：pkill -f lxx-calendar-boards-simulator"
```

---

## 注意事项

1. **端口占用**: 如果端口 8080 被占用，可指定其他端口
2. **仅本地访问**: 服务器绑定 `127.0.0.1`，外部网络无法访问
3. **Deep Sleep 循环**: 模拟器每 60 秒重启一次 embassy 执行器，HTTP 服务器保持运行
4. **配置文件**: 保存在 `/tmp/simulator_flash.bin`

---

## 环境变量

| 变量名 | 默认值 | 说明 |
|--------|--------|------|
| `SIMULATOR_PORT` | `8080` | HTTP 服务器端口 |
| `RUST_LOG` | `info` | 日志级别 (error/warn/info/debug/trace) |

```bash
# 使用 debug 日志级别
RUST_LOG=debug SIMULATOR_PORT=9000 cargo rs
```

---

## 常见问题

### Q: 模拟器无法启动？
A: 检查端口是否被占用，尝试指定其他端口：
```bash
SIMULATOR_PORT=9000 cargo rs
```

### Q: HTTP 请求返回连接被拒绝？
A: 模拟器可能正在 Deep Sleep 重启周期，等待 2-3 秒后重试。

### Q: 如何查看模拟器日志？
A: 启动时重定向日志：
```bash
cargo rs > simulator.log 2>&1 &
tail -f simulator.log
```

### Q: Deep Sleep 周期是多久？
A: 默认 60 秒。修改代码中的 `embassy_time::Duration::from_secs(60)` 可调整。
