# Cargo 构建系统

## 项目结构

```
lxx-calendar/
├── Cargo.toml                  # Workspace 配置
├── .cargo/
│   └── config.toml            # 构建配置和别名
├── lxx-calendar-core/         # 主程序
├── lxx-calendar-common/       # 公共抽象层
├── lxx-calendar-graphics/     # 图形资源
├── lxx-calendar-quotes/       # 格言库
├── lxx-calendar-boards/       # 板级支持包
│   ├── esp32c6/              # ESP32-C6 硬件平台
│   ├── tspi/                 # 泰山派 Linux 平台
│   └── simulator/            # PC 模拟器平台
└── libs/                     # 外部依赖
```

## 构建目标

| 平台 | 目标架构 | Rust Target | 包名 |
|------|----------|-------------|------|
| ESP32-C6 | RISC-V 32位 | `riscv32imac-unknown-none-elf` | `lxx-calendar-boards-esp32c6` |
| 泰山派 | ARM64 Linux | `aarch64-unknown-linux-gnu` | `lxx-calendar-boards-tspi` |
| 模拟器 | x86_64 Linux | `x86_64-unknown-linux-gnu` | `lxx-calendar-boards-simulator` |
| 模拟器(Windows) | x86_64 Windows | `x86_64-pc-windows-gnu` | `lxx-calendar-boards-simulator` |

## Cargo Alias 命令

### ESP32-C6

```bash
# 构建
cargo besp

# Release 构建
cargo bespr
```

### 泰山派 (tspi)

```bash
# 构建
cargo bspi

# Release 构建
cargo btspir
```

### 模拟器 (simulator) - Linux

```bash
# 构建（无图形）
cargo bs

# Release 构建
cargo bsr

# 构建（带SDL2图形）
cargo bsg

# Release 构建
cargo bsgr

# 运行
cargo rs

# Release 运行
cargo rsr

# 运行（带图形）
cargo rsg

# Release 运行（带图形）
cargo rsgr
```

## 常见问题

### ❌ 为什么不能用 `cargo check`？

**问题表现：**
```bash
cargo check
# 编译错误：esp-wifi-sys 找不到 VaargType
```

**根本原因：**
1. `cargo check` 会编译**所有** workspace 成员，包括嵌入式目标
2. 嵌入式依赖（如 `esp-wifi-sys`）需要完整的 ESP-IDF C 代码环境
3. 某些 target 的配置（linker script、rustflags）未正确设置

**正确做法：**
- ✅ 使用针对特定平台的 `cargo <alias>` 命令
- ✅ `bespr`：ESP32-C6 Release 构建
- ✅ `bs`：模拟器构建（适用于开发调试）
- ❌ 不要使用 `cargo check`，它不适合嵌入式项目

### ✅ 什么时候用哪个命令？

| 场景 | 推荐命令 | 说明 |
|------|---------|------|
| **开发 ESP32-C6 硬件** | `cargo bespr` | 编译用于烧录的 Release 版本 |
| **开发 ESP32-C6 硬件（调试）** | `cargo besp` | 编译 Debug 版本，便于断点调试 |
| **开发模拟器** | `cargo bs` | 快速编译，适合日常开发 |
| **最终测试** | `cargo bespr` / `cargo bsr` | 生产环境版本 |

### 📚 相关文档
- [系统架构概览](./01-系统架构概览.md) - 了解项目支持的平台
- [关键时序要求](./06-关键时序要求.md) - 了解不同模式的时序差异

## Features 开关

各板级支持包的 Feature 定义：

| 包 | Feature | 说明 |
|----|---------|------|
| `lxx-calendar-boards-esp32c6` | `esp32c6` | ESP32-C6 平台硬件 |
| `lxx-calendar-boards-tspi` | `tspi` | 泰山派 Linux 平台 |
| `lxx-calendar-boards-simulator` | `simulator` | PC 模拟器平台 |
| `lxx-calendar-boards-simulator` | `embedded_graphics_simulator` | 模拟器 SDL2 图形支持 |

## 依赖管理

项目使用 `workspace.dependencies` 统一管理依赖版本，所有 crate 共享相同的依赖版本。

### 核心依赖

- `embassy-executor` - 异步执行器
- `embassy-sync` - 同步原语
- `embassy-time` - 时间管理
- `embedded-hal` - 硬件抽象层
- `log` / `defmt` - 日志系统

### 平台依赖

- **ESP32-C6**: `esp-hal`, `esp-rtos`, `esp-radio`
- **泰山派**: Linux 系统调用 (`linux-embedded-hal`)
- **模拟器**: `embedded-graphics-simulator`
