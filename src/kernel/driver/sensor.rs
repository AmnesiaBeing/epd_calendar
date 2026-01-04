// src/driver/sensor.rs
use crate::common::error::{AppError, Result};

#[cfg(feature = "embedded_esp")]
use esp_hal::peripherals::Peripherals;

pub trait SensorDriver {
    /// 读取传感器数据
    async fn get_humidity(&mut self) -> Result<i32>;

    /// 获取温度
    async fn get_temperature(&mut self) -> Result<i32>;
}

/// 模拟传感器驱动
#[cfg(any(feature = "simulator", feature = "embedded_linux"))]
pub struct MockSensorDriver {
    temperature: i32,
    humidity: i32,
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
    async fn get_humidity(&mut self) -> Result<i32> {
        Ok(self.humidity)
    }

    async fn get_temperature(&mut self) -> Result<i32> {
        Ok(self.temperature)
    }
}

/// ESP32传感器驱动（读取SHT20温湿度传感器）
#[cfg(feature = "embedded_esp")]
pub struct EspSensorDriver {
    sht20:
        sht25::Sht25<esp_hal::i2c::master::I2c<'static, esp_hal::Blocking>, esp_hal::delay::Delay>,
}

#[cfg(feature = "embedded_esp")]
impl EspSensorDriver {
    /// 创建新的ESP32传感器驱动实例
    ///
    /// # 参数
    /// - `peripherals`: ESP32外设实例
    ///
    /// # 返回值
    /// - `Result<EspSensorDriver>`: 新的ESP32传感器驱动实例
    pub fn new(peripherals: &Peripherals) -> Result<Self> {
        use esp_hal::{i2c::master::Config, time::Rate};

        log::info!("Initializing ESP sensor driver with SHT20");

        // 配置I2C总线
        let scl = unsafe { peripherals.GPIO10.clone_unchecked() };
        let sda = unsafe { peripherals.GPIO11.clone_unchecked() };

        let i2c = esp_hal::i2c::master::I2c::new(
            unsafe { peripherals.I2C0.clone_unchecked() },
            Config::default().with_frequency(Rate::from_khz(100)),
        )
        .map_err(|e| {
            log::error!("Failed to initialize I2C: {:?}", e);
            AppError::SensorError
        })?
        .with_scl(scl)
        .with_sda(sda);

        // 初始化SHT20传感器
        let sht20 = sht25::Sht25::new(i2c, esp_hal::delay::Delay::new())
            .map_err(|_| AppError::SensorError)?;

        Ok(Self { sht20 })
    }
}

#[cfg(feature = "embedded_esp")]
impl SensorDriver for EspSensorDriver {
    async fn get_humidity(&mut self) -> Result<i32> {
        use embassy_time::Timer;
        self.sht20
            .trigger_rh_measurement()
            .map_err(|_| AppError::SensorError)?;
        Timer::after(embassy_time::Duration::from_millis(15)).await; // TODO: 后续需要修改延时，应当根据采样率进行调整

        match self.sht20.read_rh().map_err(|_| AppError::SensorError) {
            Ok(humidity) => {
                log::debug!("Humidity: {}%", humidity);
                Ok(humidity)
            }
            Err(e) => {
                log::error!("Failed to read humidity: {:?}", e);
                Err(AppError::SensorError)
            }
        }
    }

    async fn get_temperature(&mut self) -> Result<i32> {
        use embassy_time::Timer;
        self.sht20
            .trigger_temp_measurement()
            .map_err(|_| AppError::SensorError)?;
        Timer::after(embassy_time::Duration::from_millis(15)).await; // TODO: 后续需要修改延时，应当根据采样率进行调整

        match self.sht20.read_temp().map_err(|_| AppError::SensorError) {
            Ok(temperature) => {
                log::debug!("Temperature: {}°C", temperature);
                Ok(temperature)
            }
            Err(e) => {
                log::error!("Failed to read temperature: {:?}", e);
                Err(AppError::SensorError)
            }
        }
    }
}

#[cfg(any(feature = "simulator", feature = "embedded_linux"))]
pub type DefaultSensorDriver = MockSensorDriver;

#[cfg(feature = "embedded_esp")]
pub type DefaultSensorDriver = EspSensorDriver;
