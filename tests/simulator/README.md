# 模拟器测试文档

## 测试环境

- **目标平台**: x86_64-unknown-linux-gnu (PC 模拟器)
- **运行命令**: `cargo rs` 或 `cargo bsg`（带图形）
- **HTTP 服务端口**: 8080（默认，可通过环境变量 SIMULATOR_PORT 修改）

## ⚠️ 重要：测试前准备

### 1. 创建虚拟网络设备（必须）

在运行需要网络的测试之前，必须先告知用户执行 `tap.sh` 创建虚拟网络设备：

```bash
# 需要 root 权限
sudo bash ./tap.sh

# 验证创建成功
ip link show tap99
```

**注意**：
- 如果不执行此脚本，网络相关测试将失败（模拟器会优雅降级到离线模式）

### 2. 启动模拟器

```bash
# 进入项目目录
cd /home/zzh/.zeroclaw/workspace/epd_calendar

# 编译并运行模拟器（后台运行，详细日志）
RUST_LOG=debug cargo rs > simulator.log 2>&1 &

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
| `/api/ble/config` | POST | 模拟 BLE 配置下发 | 见下方示例 |

## BLE 配置 API 示例

### WiFi 配置
```json
{
  "type": "wifi_config",
  "data": {
    "wifi_ssid": "TestWiFi",
    "wifi_password": "12345678"
  }
}
```

### 网络配置
```json
{
  "type": "network_config",
  "data": {
    "location_id": "101010101",
    "sync_interval_minutes": 60,
    "auto_sync": true
  }
}
```

### 系统命令
```json
{
  "type": "command",
  "data": {
    "action": "network_sync"
  }
}
```

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

### 4. 配置持久化测试

| 测试项 | 预期结果 | 验证方法 |
|--------|----------|----------|
| 首次启动 | 加载默认配置 | 删除 Flash 文件后重启 |
| BLE 配置保存 | 配置写入 Flash | `POST /api/ble/config` |
| 重启后恢复 | 配置保留 | 重启模拟器验证 |
| CRC32 校验 | 校验和验证通过 | 检查 Flash 文件 |
| 恢复出厂设置 | 配置被擦除 | 删除 Flash 文件模拟 |

### 5. 配置更新测试

| 测试项 | 预期结果 | 验证方法 |
|--------|----------|----------|
| WiFi 配置更新 | SSID/密码保存 | BLE 发送配置 |
| 网络配置更新 | 位置/同步间隔保存 | BLE 发送配置 |
| 配置字段边界 | 边界值验证 | 超长 SSID 测试 |
| 快速连续更新 | 无数据丢失 | 连续发送 5 次配置 |

### 6. 配置完整性测试

| 测试项 | 预期结果 | 验证方法 |
|--------|----------|----------|
| Flash 文件结构 | Magic/版本/校验和正确 | 解析二进制文件 |
| 配置大小限制 | 不超过 1024 字节 | 检查文件大小 |
| 损坏配置检测 | 降级到默认配置 | 修改 CRC 后重启 |
| 版本不匹配 | 使用默认配置 | 修改版本号后重启 |
| Magic 错误 | 使用默认配置 | 修改 Magic 后重启 |

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
python3 tests/simulator/test_config_persistence.py
python3 tests/simulator/test_config_update.py
python3 tests/simulator/test_config_integrity.py
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
- Flash 文件正确创建和更新
- 配置在重启后保留

### 配置测试特定标志

- **Magic Number**: 0x4C585843 ('LXXC')
- **版本号**: 1
- **CRC32**: 与配置数据匹配
- **配置大小**: ≤ 1024 字节
- **重启后**: 日志显示 "Config loaded from storage, version: 1"

### 常见问题

1. **连接被拒绝**: 模拟器未启动或端口错误
2. **网络同步失败**: 
   - 未执行 `sudo ./tap.sh`
   - 网络权限不足
3. **超时**: 模拟器卡死，需要重启
4. **JSON 解析失败**: API 返回格式异常
5. **配置未保存**: 
   - Flash 文件权限问题
   - 磁盘空间不足
   - BLE 配置未正确接收
6. **配置重启后丢失**: 
   - CRC 校验失败
   - 版本不匹配
   - Magic Number 错误
   - Flash 损坏

### tap.sh 说明

`tap.sh` 脚本用于创建虚拟网络设备 `tap99`，使模拟器能够进行网络通信。

**用户创建设备：**
```bash
sudo bash ./tap.sh
```

**注意事项：**
- 需要告知用户需要 root 权限，让用户手动执行

## 运行所有配置测试

```bash
# 快速运行所有配置测试
python3 tests/simulator/test_config_persistence.py && \
python3 tests/simulator/test_config_update.py && \
python3 tests/simulator/test_config_integrity.py
```

## 查看配置测试日志

```bash
# 实时查看模拟器日志（包含配置操作）
tail -f simulator.log | grep -E "(Config|Flash|BLE)"

# 查看配置加载日志
grep "Config" simulator.log

# 查看 Flash 操作日志
grep "Flash" simulator.log
```

## 配置持久化验证

### Flash 文件位置
```
/tmp/simulator_flash.bin
```

### 配置文件结构
```
偏移 0-3:   Magic Number (0x4C585843 'LXXC')
偏移 4-7:   版本号 (当前为 1)
偏移 8-11:  CRC32 校验和
偏移 12-31: 保留字段
偏移 32+:   Postcard 序列化的配置数据
```

### 完整测试流程

```bash
# 1. 清理旧配置
rm -f /tmp/simulator_flash.bin

# 2. 启动模拟器（首次启动，应加载默认配置）
cargo rs > simulator.log 2>&1 &
sleep 3

# 3. 通过 BLE 设置配置
curl -X POST http://127.0.0.1:8080/api/ble/connect
curl -X POST http://127.0.0.1:8080/api/ble/config \
  -H "Content-Type: application/json" \
  -d '{"type":"wifi_config","data":{"wifi_ssid":"MyWiFi","wifi_password":"password123"}}'

# 4. 验证配置已保存
ls -lh /tmp/simulator_flash.bin
hexdump -C /tmp/simulator_flash.bin | head -5

# 5. 重启模拟器
pkill -f lxx-calendar-boards-simulator
sleep 2
cargo rs > simulator2.log 2>&1 &
sleep 3

# 7. 验证配置完整性
python3 tests/simulator/test_config_integrity.py

# 8. 清理
pkill -f lxx-calendar-boards-simulator
```
