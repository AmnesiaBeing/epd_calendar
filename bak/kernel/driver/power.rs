// src/driver/power.rs

/// 电源管理模块
///
/// 本模块定义了电源监控功能，支持不同平台的电源状态检测
/// 包括电池电量监控、电源状态变化检测等
use crate::common::error::Result;
#[cfg(feature = "esp32c6")]
use esp_hal::{
    Async,
    analog::adc::{Adc, AdcPin},
    gpio::Input,
    peripherals::{GPIO2, Peripherals},
};

/// 电池挡位枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BatteryLevel {
    /// Level0: 电量极低（<20%）
    Level0,
    /// Level1: 电量低（20%-40%）
    Level1,
    /// Level2: 电量中等（40%-60%）
    Level2,
    /// Level3: 电量较高（60%-80%）
    Level3,
    /// Level4: 电量充足（80%-100%）
    Level4,
}

/// 电源管理trait
///
/// 定义电源监控的通用接口，支持不同平台的实现
pub trait PowerDriver {
    /// 获取电池挡位
    ///
    /// # 返回值
    /// - `Result<BatteryLevel>`: 电池挡位
    async fn battery_level(&mut self) -> Result<BatteryLevel>;

    /// 检查是否正在充电
    ///
    /// # 返回值
    /// - `Result<bool>`: 是否正在充电
    async fn is_charging(&self) -> Result<bool>;
}

/// Mock电源驱动实现
///
/// 用于测试和模拟环境的电源驱动实现
#[cfg(any(feature = "simulator", feature = "tspi"))]
pub struct MockPowerDriver {
    /// 模拟电池电量
    battery_level: BatteryLevel,
    /// 模拟充电状态
    charging: bool,
}

#[cfg(any(feature = "simulator", feature = "tspi"))]
impl MockPowerDriver {
    pub fn new() -> Self {
        Self {
            battery_level: BatteryLevel::Level4,
            charging: false,
        }
    }
}

#[cfg(any(feature = "simulator", feature = "tspi"))]
impl PowerDriver for MockPowerDriver {
    async fn battery_level(&mut self) -> Result<BatteryLevel> {
        Ok(self.battery_level)
    }

    async fn is_charging(&self) -> Result<bool> {
        Ok(self.charging)
    }
}

/// ESP32平台电源驱动实现
#[cfg(feature = "esp32c6")]
pub struct EspPowerDriver {
    power_adc: Adc<'static, esp_hal::peripherals::ADC1<'static>, Async>,
    power_adc_pin: AdcPin<GPIO2<'static>, esp_hal::peripherals::ADC1<'static>>,
    charging_pin: Input<'static>,
}

#[cfg(feature = "esp32c6")]
impl EspPowerDriver {
    /// 创建新的ESP32电源驱动实例
    ///
    /// # 参数
    /// - `peripherals`: ESP32外设实例
    ///
    /// # 返回值
    /// - `Result<EspPowerDriver>`: 新的ESP32电源驱动实例
    pub fn new(peripherals: &Peripherals) -> Result<Self> {
        use esp_hal::{
            analog::adc::{AdcConfig, Attenuation},
            gpio::{InputConfig, Pull},
        };

        log::info!("Initializing ESP power driver");

        // 配置GPIO9为输入（充电状态检测）
        let charging_pin = unsafe { peripherals.GPIO9.clone_unchecked() };
        let charging_pin = Input::new(charging_pin, InputConfig::default().with_pull(Pull::Up));

        // 配置GPIO2为ADC输入（电池电压检测）
        let mut adc_config = AdcConfig::new();
        let power_adc_pin = adc_config.enable_pin(
            unsafe { peripherals.GPIO2.clone_unchecked() },
            Attenuation::_11dB,
        );
        let power_adc =
            Adc::new(unsafe { peripherals.ADC1.clone_unchecked() }, adc_config).into_async();

        Ok(Self {
            power_adc,
            power_adc_pin,
            charging_pin,
        })
    }
}

#[cfg(feature = "esp32c6")]
impl PowerDriver for EspPowerDriver {
    /// 获取ESP32电池电量
    ///
    /// # 返回值
    /// - `Result<u8>`: 电池电量百分比（0-100）
    async fn battery_level(&mut self) -> Result<BatteryLevel> {
        let adc_value: u16 = self.power_adc.read_oneshot(&mut self.power_adc_pin).await;

        // 转换为电压值（11dB衰减，参考电压3.9V）
        let voltage = (adc_value as f32) * 3.9 / 4095.0;

        // 根据分压电阻计算实际电池电压
        // 分压电阻：100kΩ和470kΩ
        // ADC_VIN = VBAT * (470 / (470 + 100))
        // VBAT = ADC_VIN * (570 / 470)
        let vbat = voltage * (570.0 / 470.0);

        // 锂电池电压范围：3.0V - 4.2V
        let min_voltage = 3.0;
        let max_voltage = 4.2;

        let percentage = if vbat <= min_voltage {
            0
        } else if vbat >= max_voltage {
            100
        } else {
            let percentage = ((vbat - min_voltage) / (max_voltage - min_voltage)) * 100.0;
            percentage.clamp(0.0, 100.0) as u8
        };

        let level = match percentage {
            0..=19 => BatteryLevel::Level0,
            20..=39 => BatteryLevel::Level1,
            40..=59 => BatteryLevel::Level2,
            60..=79 => BatteryLevel::Level3,
            _ => BatteryLevel::Level4,
        };
        Ok(level)
    }

    /// 检查ESP32充电状态
    ///
    /// GPIO9为低电平时表示正在充电
    ///
    /// # 返回值
    /// - `Result<bool>`: 是否正在充电
    async fn is_charging(&self) -> Result<bool> {
        use esp_hal::gpio::Level;

        let level = self.charging_pin.level();
        // 低电平表示正在充电
        Ok(level == Level::Low)
    }
}

/// 默认电源驱动类型别名
///
/// 根据平台特性选择不同的电源驱动实现
#[cfg(feature = "esp32c6")]
pub type DefaultPowerDriver = EspPowerDriver;

#[cfg(any(feature = "simulator", feature = "tspi"))]
pub type DefaultPowerDriver = MockPowerDriver;
