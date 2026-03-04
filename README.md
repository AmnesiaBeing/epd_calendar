# ePaper Calendar - 嵌入式电子墨水屏万年历 🦀

基于 Rust + Embassy 框架的嵌入式电子墨水屏万年历项目，运行在 ESP32-C6 平台上。

## 📋 功能特性

- 📅 **农历/公历显示** - 支持中国传统农历、节气、法定节假日
- 🌤️ **天气显示** - 联网获取实时天气信息
- 💬 **每日一言** - 显示精选名言/诗句
- ⏰ **闹钟/提醒** - 支持整点报时等功能
- 🔋 **低功耗设计** - 使用 Embassy 异步框架优化功耗
- 🎨 **电子墨水屏** - 低功耗、护眼显示

## 🏗️ 项目结构

```
epd_calendar/
├── lxx-calendar-core/          # 核心逻辑和状态机
├── lxx-calendar-common/        # 通用类型和 trait 定义
├── lxx-calendar-graphics/      # 图形渲染和字体/图标
├── lxx-calendar-quotes/        # 名言/一言功能
├── lxx-calendar-boards/        # 板级支持包
│   ├── esp32c6/               # ESP32-C6 目标硬件
│   ├── tspi/                  # Linux 目标 (树莓派等)
│   └── simulator/             # 桌面模拟器
└── libs/                       # 底层库
    ├── epd/                   # 电子墨水屏驱动
    ├── sxtwl-rs/              # 农历/日历计算库
    └── ...
```

## 🛠️ 开发环境

### 必需工具

```bash
# Rust 工具链 
rustup target add riscv32imac-unknown-none-elf

# ESP32 工具
cargo install espflash
cargo install probe-rs-tools

# 交叉编译工具 (Linux 目标)
# Ubuntu/Debian
sudo apt install gcc-aarch64-linux-gnu
```

### 构建命令

```bash
# ESP32-C6 (开发目标)
cargo besp      # 构建 debug 版本
cargo bespr     # 构建 release 版本

# 模拟器 (桌面测试)
cargo rs        # 运行模拟器
cargo rsr       # 运行模拟器 (release)

# Linux 目标 (树莓派等)
cargo btspi     # 构建
cargo btspir    # 构建 (release)
```

## 🔌 硬件要求

- ESP32-C6 开发板
- 电子墨水屏 (7.5 寸，YRD0750RYF665F60)
- RTC 模块 (可选，ESP32 内置)
- 蜂鸣器 (整点报时)
- 按钮 (交互控制)

## 📝 配置

项目使用 `.env` 文件管理配置（需要自行创建）：

```bash
# 天气 API 配置
WEATHER_API_KEY=your_api_key
WEATHER_LOCATION=beijing

# 网络配置
WIFI_SSID=your_ssid
WIFI_PASSWORD=your_password
```

## 🧪 测试

```bash
# 运行单元测试
cargo test

# 模拟器测试
cargo rs --features embedded_graphics_simulator
```

## 📚 相关资源

- [Embassy 文档](https://embassy.dev/)
- [ESP-RS 项目](https://esp-rs.github.io/)
- [embedded-graphics](https://embedded-graphics.com/)

## 🤝 贡献

欢迎提交 Issue 和 Pull Request！

## 📄 许可证

MIT License
