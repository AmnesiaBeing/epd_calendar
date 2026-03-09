# Flash 分区布局

## 概述

本文档描述 ESP32-C6 平台的 4MB Flash 分区布局，支持配置存储、日志存储和 OTA 更新。

## 分区表

| 分区名称 | 类型 | 偏移地址 | 大小 | 说明 |
|---------|------|---------|------|------|
| Bootloader | - | 0x00000 | 28KB | ESP-IDF 引导程序 |
| Partition Table | - | 0x08000 | 4KB | 分区表定义 |
| NVS | data/nvs | 0x09000 | 24KB | WiFi/系统配置 |
| PHY Init | data/phy | 0x0F000 | 4KB | RF 校准数据 |
| **App Config A** | data/nvs | 0x10000 | 8KB | 主配置存储 |
| **App Config B** | data/nvs | 0x12000 | 8KB | 备份配置存储 |
| **Log Storage** | data/spiffs | 0x14000 | 48KB | 循环日志存储 |
| Factory App | app/factory | 0x20000 | 1MB | 出厂固件 |
| **OTA_0** | app/ota_0 | 0x120000 | 1MB | OTA 分区 0 |
| **OTA_1** | app/ota_1 | 0x220000 | 1MB | OTA 分区 1 |
| OTA State | data/ota | 0x320000 | 8KB | OTA 启动状态 |
| Reserved | - | 0x322000 | ~888KB | 预留区域 |

## 内存映射图

```
0x00000 ┌─────────────────┐
        │   Bootloader    │  28KB
0x07000 ├─────────────────┤
        │    Reserved     │  4KB
0x08000 ├─────────────────┤
        │ Partition Table │  4KB
0x09000 ├─────────────────┤
        │       NVS       │  24KB
0x0F000 ├─────────────────┤
        │    PHY Init     │  4KB
0x10000 ├─────────────────┤
        │  App Config A   │  8KB   ─┐
0x12000 ├─────────────────┤         │ 配置区 (磨损均衡)
        │  App Config B   │  8KB   ─┘
0x14000 ├─────────────────┤
        │   Log Storage   │  48KB  ← 循环日志
0x20000 ├─────────────────┤
        │  Factory App    │  1MB
0x120000├─────────────────┤
        │     OTA_0       │  1MB   ─┐
0x220000├─────────────────┤         │ OTA 区 (A/B 更新)
        │     OTA_1       │  1MB   ─┘
0x320000├─────────────────┤
        │   OTA State     │  8KB
0x322000├─────────────────┤
        │    Reserved     │  ~888KB
0x400000└─────────────────┘
```

## 功能模块

### 1. 配置存储 (双区磨损均衡)

配置存储使用双区机制实现磨损均衡：

- **Config A** (0x10000) 和 **Config B** (0x12000) 交替使用
- 每次保存配置时写入非活动区
- 通过 `active` 标志切换当前活动区
- 包含 Magic Number、版本号、CRC32 校验

**数据结构：**
```
偏移 0-3:   Magic Number (0x4C585843 'LXXC')
偏移 4-7:   版本号 (当前为 1)
偏移 8-11:  CRC32 校验和
偏移 12-15: Active 标志 (0x41435456 'ACTV' 或 0xFFFFFFFF)
偏移 16-31: 保留字段
偏移 32+:   Postcard 序列化的配置数据
```

### 2. 日志存储 (循环缓冲区)

日志存储采用循环缓冲区设计：

- **位置**: 0x14000, 大小 48KB
- 每条日志包含：时间戳、日志级别、消息内容
- 写满后自动覆盖最旧的日志
- 支持 Error/Warn/Info/Debug/Trace 五个级别

**日志条目格式：**
```
偏移 0-3:   Magic (0x4C4F4745 'LOGE')
偏移 4-7:   时间戳 (Unix 时间)
偏移 8:     日志级别 (0-4)
偏移 9:     消息长度
偏移 10-11: 校验和
偏移 12+:   消息内容 (最大 256 字节)
```

### 3. OTA 分区

支持 A/B 双分区 OTA 更新：

- **OTA_0** (0x120000): 1MB
- **OTA_1** (0x220000): 1MB
- **OTA State** (0x320000): 存储启动分区选择

## 代码使用

### Flash 布局常量

```rust
use lxx_calendar_common::flash_layout;

// 配置分区
let config_a = flash_layout::CONFIG_A_OFFSET;  // 0x10000
let config_b = flash_layout::CONFIG_B_OFFSET;  // 0x12000

// 日志分区
let log_offset = flash_layout::LOG_OFFSET;      // 0x14000
let log_size = flash_layout::LOG_SIZE;          // 48KB

// OTA 分区
let ota_0 = flash_layout::OTA_0_OFFSET;         // 0x120000
let ota_1 = flash_layout::OTA_1_OFFSET;         // 0x220000
```

### 配置持久化

```rust
use lxx_calendar_common::storage::{ConfigPersistence, FlashDevice};

let mut persistence = ConfigPersistence::new(flash);

// 加载配置
let config = persistence.load_config::<SystemConfig>().await?;

// 保存配置 (自动切换到另一分区)
persistence.save_config(&config).await?;

// 恢复出厂设置
persistence.factory_reset().await?;
```

### 日志存储

```rust
use lxx_calendar_common::storage::{LogStorage, LogLevel};

let mut log_storage = LogStorage::new(flash);
log_storage.initialize().await?;

// 写入日志
log_storage.write(timestamp, LogLevel::Info, b"System started").await?;

// 读取日志
let entries = log_storage.read_entries(10).await?;

// 清空日志
log_storage.clear().await?;
```

## 安全考虑

1. **磨损均衡**: 配置存储使用双区交替写入，延长 Flash 寿命
2. **数据完整性**: CRC32 校验确保数据正确性
3. **版本兼容**: 版本号检查防止加载不兼容的配置
4. **原子写入**: 先写数据区，再切换活动标志

## 烧录命令

```bash
# 烧录分区表
esptool.py --chip esp32c6 write_flash 0x8000 partitions.bin

# 烧录固件到 Factory 分区
esptool.py --chip esp32c6 write_flash 0x20000 firmware.bin

# 烧录固件到 OTA_0 分区
esptool.py --chip esp32c6 write_flash 0x120000 firmware.bin
```