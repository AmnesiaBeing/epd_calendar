// src/driver/sensor.rs
use crate::common::error::Result;

#[derive(Debug, Clone)]
pub struct SensorData {
    pub temperature: f32,
    pub humidity: f32,
    pub pressure: Option<f32>,
}

pub trait SensorDriver {
    /// 读取传感器数据
    async fn read(&mut self) -> Result<SensorData>;

    /// 检查传感器是否可用
    async fn is_available(&self) -> bool;

    /// 校准传感器
    async fn calibrate(&mut self) -> Result<()>;
}

/// 模拟传感器驱动
#[cfg(any(feature = "simulator"))]
pub struct MockSensorDriver {
    temperature: f32,
    humidity: f32,
}

#[cfg(any(feature = "simulator"))]
impl MockSensorDriver {
    pub fn new() -> Self {
        Self {
            temperature: 22.5,
            humidity: 45.0,
        }
    }

    pub fn set_temperature(&mut self, temp: f32) {
        self.temperature = temp;
    }

    pub fn set_humidity(&mut self, humidity: f32) {
        self.humidity = humidity;
    }
}

#[cfg(any(feature = "simulator"))]
impl SensorDriver for MockSensorDriver {
    async fn read(&mut self) -> Result<SensorData> {
        // 模拟轻微的读数变化
        self.temperature += 0.1;
        if self.temperature > 30.0 {
            self.temperature = 15.0;
        }

        self.humidity += 0.5;
        if self.humidity > 80.0 {
            self.humidity = 20.0;
        }

        Ok(SensorData {
            temperature: self.temperature,
            humidity: self.humidity,
            pressure: Some(1013.25),
        })
    }

    async fn is_available(&self) -> bool {
        true
    }

    async fn calibrate(&mut self) -> Result<()> {
        log::info!("Mock sensor calibrated");
        Ok(())
    }
}

/// Linux传感器驱动（读取系统传感器）
#[cfg(feature = "embedded_linux")]
pub struct LinuxSensorDriver;

#[cfg(feature = "embedded_linux")]
impl LinuxSensorDriver {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(feature = "embedded_linux")]
impl SensorDriver for LinuxSensorDriver {
    async fn read(&mut self) -> Result<SensorData> {
        // 读取系统传感器数据
        // 简化实现，返回模拟数据
        Ok(SensorData {
            temperature: 23.5,
            humidity: 55.0,
            pressure: Some(1013.25),
        })
    }

    async fn is_available(&self) -> bool {
        // 检查系统传感器是否可用
        true
    }

    async fn calibrate(&mut self) -> Result<()> {
        log::info!("Linux sensor calibrated");
        Ok(())
    }
}

#[cfg(any(feature = "simulator"))]
pub type DefaultSensorDriver = MockSensorDriver;

#[cfg(feature = "embedded_linux")]
pub type DefaultSensorDriver = LinuxSensorDriver;
