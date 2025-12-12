// src/driver/sensor.rs
use crate::common::error::Result;

pub trait SensorDriver {
    /// 读取传感器数据
    async fn get_humidity(&mut self) -> Result<u8>;

    /// 获取温度
    async fn get_temperature(&mut self) -> Result<u8>;
}

/// 模拟传感器驱动
#[cfg(any(feature = "simulator", feature = "embedded_linux"))]
pub struct MockSensorDriver {
    temperature: u8,
    humidity: u8,
}

#[cfg(any(feature = "simulator", feature = "embedded_linux"))]
impl MockSensorDriver {
    pub fn new() -> Self {
        Self {
            temperature: 22,
            humidity: 45,
        }
    }
}

#[cfg(any(feature = "simulator", feature = "embedded_linux"))]
impl SensorDriver for MockSensorDriver {
    async fn get_humidity(&mut self) -> Result<u8> {
        Ok(self.humidity)
    }

    async fn get_temperature(&mut self) -> Result<u8> {
        Ok(self.temperature)
    }
}

// TODO: 真正实现驱动

/// ESP32传感器驱动（读取系统传感器）
#[cfg(feature = "embedded_esp")]
#[derive(Debug)]
pub struct EspSensorDriver;

#[cfg(feature = "embedded_esp")]
impl EspSensorDriver {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(feature = "embedded_esp")]
impl SensorDriver for EspSensorDriver {
    async fn get_humidity(&mut self) -> Result<u8> {
        // 读取系统传感器数据
        // 简化实现，返回模拟数据
        Ok(55)
    }

    async fn get_temperature(&mut self) -> Result<u8> {
        // 读取系统传感器数据
        // 简化实现，返回模拟数据
        Ok(23)
    }
}

#[cfg(any(feature = "simulator", feature = "embedded_linux"))]
pub type DefaultSensorDriver = MockSensorDriver;

#[cfg(feature = "embedded_esp")]
pub type DefaultSensorDriver = EspSensorDriver;
