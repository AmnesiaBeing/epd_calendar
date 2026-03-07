# 模拟器测试文档

## 测试环境

- **目标平台**: x86_64-unknown-linux-gnu (PC 模拟器)
- **运行命令**: `cargo rs` 或 `cargo bsg`（带图形）
- **HTTP 服务端口**: 8080（默认，可通过环境变量 SIMULATOR_PORT 修改）

## ⚠️ 重要：测试前准备

### 1. 创建虚拟网络设备（必须）

在运行需要网络的测试之前，必须先执行 `tap.sh` 创建虚拟网络设备：

```bash
# 需要 root 权限
sudo ./tap.sh

# 验证创建成功
ip link show tap99
```

**注意**：
- 如果不执行此脚本，网络相关测试将失败（模拟器会优雅降级到离线模式）
- 测试完成后可以删除虚拟设备：`sudo ip link del tap99`

### 2. 启动模拟器

```bash
# 进入项目目录
cd /home/zzh/.zeroclaw/workspace/epd_calendar

# 编译并运行模拟器（后台运行）
cargo rs > simulator.log 2>&1 &

# 等待服务启动
sleep 3
```

### 3. 验证服务

```bash
# 检查服务是否启动
curl -s http://127.0.0.1:8080/status
```

## ⚠️ 测试执行注意事项

1. **必须顺序执行测试**：每个测试之间建议等待 2-3 秒，让模拟器完成状态转换
2. **不能并行运行**：同时运行多个测试会导致状态竞争，建议逐个运行
3. **网络测试需要 tap 设备**：时间同步和天气同步测试需要先执行 `sudo ./tap.sh`

## HTTP API 端点

| 端点 | 方法 | 说明 | 请求体示例 |
|------|------|------|------------|
| `/status` | GET | 获取系统整体状态 | - |
| `/status/rtc` | GET | 获取 RTC 状态 | - |
| `/status/ble` | GET | 获取 BLE 状态 | - |
| `/status/watchdog` | GET | 看门狗状态 | - |
| `/api/button` | POST | 模拟按键事件 | `{"event": "short_press"}` |
| `/api/ble/connect` | POST | 模拟 BLE 连接 | - |
| `/api/ble/disconnect` | POST | 模拟 BLE 断开 | - |
| `/api/ble/config` | POST | 模拟 BLE 配置下发 | `{"data": {"ssid": "TestWiFi", "password": "12345678"}}` |

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
| 网络同步 | 时间同步成功 | **需要 tap 设备**，查看日志 |

### 4. 用户交互测试

| 测试项 | 请求体 | 说明 |
|--------|--------|------|
| 短按 | `{"event": "short_press"}` | 进入 BLE 配网模式 |
| 双击 | `{"event": "double_click"}` | 双击事件 |
| 三击 | `{"event": "triple_click"}` | 进入配对模式 |
| 长按 | `{"event": "long_press"}` | 恢复出厂设置 |

## 运行测试

### 自动测试（推荐）

```bash
# 1. 先创建虚拟网络设备（需要 root）
sudo ./tap.sh

# 2. 等待几秒让网络设备就绪
sleep 2

# 3. 启动模拟器
cargo rs > simulator.log 2>&1 &
sleep 3

# 4. 运行测试（按顺序执行，不要并行）
python3 tests/simulator/test_basic.py
python3 tests/simulator/test_ble.py
python3 tests/simulator/test_button.py
python3 tests/simulator/test_network.py   # 需要 tap 设备

# 5. 停止模拟器
pkill -f lxx-calendar-boards-simulator
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
  -d '{"event": "short_press"}'

# 5. 模拟 BLE 连接
curl -X POST http://127.0.0.1:8080/api/ble/connect

# 6. 模拟 BLE 配置
curl -X POST http://127.0.0.1:8080/api/ble/config \
  -H "Content-Type: application/json" \
  -d '{"data": {"ssid": "TestWiFi", "password": "12345678"}}'
```

## 测试结果解读

### 成功标志

- HTTP 请求返回 200 状态码
- JSON 响应格式正确
- 终端日志无 ERROR 级别错误
- 模拟器进程稳定运行

### 常见问题

1. **连接被拒绝**: 模拟器未启动或端口错误
2. **网络同步失败**: 
   - 未执行 `sudo ./tap.sh`
   - 网络权限不足
3. **超时**: 模拟器卡死，需要重启
4. **JSON 解析失败**: API 返回格式异常

### tap.sh 说明

`tap.sh` 脚本用于创建虚拟网络设备 `tap99`，使模拟器能够进行网络通信。

**创建设备：**
```bash
sudo ./tap.sh
```

**删除设备：**
```bash
sudo ip link del tap99
```

**注意事项：**
- 需要 root 权限
- 可能需要安装 `iproute2` 和 `iptables`
- 退出测试后建议删除设备避免资源泄漏
