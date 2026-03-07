# 模拟器测试文档

## 测试环境

- **目标平台**: x86_64-unknown-linux-gnu (PC 模拟器)
- **运行命令**: `cargo rs` 或 `cargo bsg`（带图形）
- **HTTP 服务端口**: 8080（默认，可通过环境变量 SIMULATOR_PORT 修改）

## 测试前准备

### 1. 启动模拟器

```bash
# 进入项目目录
cd /home/zzh/.zeroclaw/workspace/epd_calendar

# 编译并运行模拟器（后台运行）
cargo rs > simulator.log 2>&1 &

# 等待服务启动
sleep 3
```

### 2. 验证服务

```bash
# 检查服务是否启动
curl -s http://127.0.0.1:8080/status
```

## HTTP API 端点

| 端点 | 方法 | 说明 | 请求体示例 |
|------|------|------|------------|
| `/status` | GET | 获取系统整体状态 | - |
| `/status/rtc` | GET | 获取 RTC 状态 | - |
| `/status/ble` | GET | 获取 BLE 状态 | - |
| `/status/watchdog` | GET | 看门狗状态 | - |
| `/api/button` | POST | 模拟按键事件 | `{"type": "short"}` |
| `/api/ble/connect` | POST | 模拟 BLE 连接 | - |
| `/api/ble/disconnect` | POST | 模拟 BLE 断开 | - |
| `/api/ble/config` | POST | 模拟 BLE 配置下发 | `{"ssid": "TestWiFi", "password": "12345678"}` |

## 测试用例

### 1. 基础启动流程测试

| 测试项 | 预期结果 | 验证方法 |
|--------|----------|----------|
| 编译成功 | 无编译错误 | `cargo bs` |
| 进程启动 | HTTP 服务监听 8080 端口 | `curl http://127.0.0.1:8080/status` |
| 日志输出 | 显示 "lxx-calendar starting..." | 查看终端日志 |
| 服务初始化 | 显示 "All services initialized" | 查看终端日志 |
| 进入工作状态 | 显示 "Entering normal work mode" | 查看终端日志 |

### 2. BLE 功能测试

| 测试项 | 预期结果 | 验证方法 |
|--------|----------|----------|
| BLE 未配置状态 | 显示设备名称 | `/status` 或日志 |
| BLE 连接模拟 | 状态变为 connected | `GET /status/ble` |
| BLE 断开模拟 | 状态变为 disconnected | `POST /api/ble/disconnect` |
| BLE 配置下发 | 接收到 SSID/password | 日志查看 |

### 3. 时间与同步测试

| 测试项 | 预期结果 | 验证方法 |
|--------|----------|----------|
| RTC 初始化 | 时间已设置 | `GET /status/rtc` |
| 时间显示 | 正确显示时间戳 | 返回值验证 |

### 4. 用户交互测试

| 测试项 | 请求体 | 说明 |
|--------|--------|------|
| 短按 | `{"type": "short"}` | 进入 BLE 配网模式 |
| 双击 | `{"type": "double"}` | 双击事件 |
| 三击 | `{"type": "triple"}` | 进入配对模式 |
| 长按 | `{"type": "long"}` | 恢复出厂设置 |

## 运行测试

### 自动测试（推荐）

```bash
# 运行所有测试
python3 tests/simulator/test_all.py

# 运行特定测试
python3 tests/simulator/test_basic.py
python3 tests/simulator/test_ble.py
python3 tests/simulator/test_button.py
```

### 手动测试

```bash
# 1. 测试系统状态
curl -s http://127.0.0.1:8080/status | python3 -m json.tool

# 2. 测试 RTC 状态
curl -s http://127.0.0.1:8080/status/rtc | python3 -m json.tool

# 3. 测试 BLE 状态
curl -s http://127.0.0.1:8080/status/ble | python3 -m json.tool

# 4. 模拟按键
curl -X POST http://127.0.0.1:8080/api/button \
  -H "Content-Type: application/json" \
  -d '{"type": "short"}'

# 5. 模拟 BLE 连接
curl -X POST http://127.0.0.1:8080/api/ble/connect

# 6. 模拟 BLE 配置
curl -X POST http://127.0.0.1:8080/api/ble/config \
  -H "Content-Type: application/json" \
  -d '{"ssid": "TestWiFi", "password": "12345678"}'
```

## 测试结果解读

### 成功标志

- HTTP 请求返回 200 状态码
- JSON 响应格式正确
- 终端日志无 ERROR 级别错误
- 模拟器进程稳定运行

### 常见问题

1. **连接被拒绝**: 模拟器未启动或端口错误
2. **超时**: 模拟器卡死，需要重启
3. **JSON 解析失败**: API 返回格式异常
