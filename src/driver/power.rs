// src/driver/power.rs
use crate::common::error::Result;
use crate::common::system_state::BatteryLevel;

pub trait PowerMonitor {
    /// 检查是否正在充电
    async fn is_charging(&self) -> bool;

    /// 获取电池电量等级
    async fn battery_level(&self) -> BatteryLevel;

    /// 获取电池电压（如果支持）
    async fn battery_voltage(&self) -> Result<f32>;

    /// 进入低功耗模式
    async fn enter_low_power(&self) -> Result<()>;

    /// 唤醒系统
    async fn wake(&self) -> Result<()>;
}

/// 模拟电源监控
#[cfg(any(feature = "simulator", feature = "embedded_linux"))]
pub struct MockPowerMonitor {
    charging: bool,
    level: BatteryLevel,
}

#[cfg(any(feature = "simulator", feature = "embedded_linux"))]
impl MockPowerMonitor {
    pub fn new() -> Self {
        Self {
            charging: false,
            level: BatteryLevel::Level0,
        }
    }

    pub fn set_charging(&mut self, charging: bool) {
        self.charging = charging;
    }

    pub fn set_battery_level(&mut self, level: BatteryLevel) {
        self.level = level;
    }
}

#[cfg(any(feature = "simulator", feature = "embedded_linux"))]
impl PowerMonitor for MockPowerMonitor {
    async fn is_charging(&self) -> bool {
        self.charging
    }

    async fn battery_level(&self) -> BatteryLevel {
        self.level.clone()
    }

    async fn battery_voltage(&self) -> Result<f32> {
        match self.level {
            BatteryLevel::Level0 => Ok(3.2),
            BatteryLevel::Level1 => Ok(3.5),
            BatteryLevel::Level2 => Ok(3.7),
            BatteryLevel::Level3 => Ok(3.9),
            BatteryLevel::Level4 => Ok(4.1),
        }
    }

    async fn enter_low_power(&self) -> Result<()> {
        log::info!("Mock power monitor entering low power mode");
        Ok(())
    }

    async fn wake(&self) -> Result<()> {
        log::info!("Mock power monitor waking up");
        Ok(())
    }
}

// TODO: 实现ESP32的PowerMonitor

/// 模拟电源监控
#[cfg(feature = "embedded_esp")]
pub struct MockPowerMonitor {
    charging: bool,
    level: BatteryLevel,
}

#[cfg(feature = "embedded_esp")]
impl MockPowerMonitor {
    pub fn new() -> Self {
        Self {
            charging: false,
            level: BatteryLevel::Level0,
        }
    }
}

#[cfg(feature = "embedded_esp")]
impl PowerMonitor for MockPowerMonitor {
    async fn is_charging(&self) -> bool {
        self.charging
    }

    async fn battery_level(&self) -> BatteryLevel {
        self.level.clone()
    }

    async fn battery_voltage(&self) -> Result<f32> {
        match self.level {
            BatteryLevel::Level0 => Ok(3.2),
            BatteryLevel::Level1 => Ok(3.5),
            BatteryLevel::Level2 => Ok(3.7),
            BatteryLevel::Level3 => Ok(3.9),
            BatteryLevel::Level4 => Ok(4.1),
        }
    }

    async fn enter_low_power(&self) -> Result<()> {
        log::info!("Mock power monitor entering low power mode");
        Ok(())
    }

    async fn wake(&self) -> Result<()> {
        log::info!("Mock power monitor waking up");
        Ok(())
    }
}

// Simulator和嵌入式Linux环境下的默认电源监控
#[cfg(any(feature = "simulator", feature = "embedded_linux"))]
pub type DefaultPowerMonitor = MockPowerMonitor;

// ESP32环境下的默认电源监控
#[cfg(feature = "embedded_esp")]
pub type DefaultPowerMonitor = MockPowerMonitor;
