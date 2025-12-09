//! 天气数据源模块
//! 提供天气相关数据的数据源实现

use alloc::boxed::Box;
use alloc::format;
use async_trait::async_trait;
use core::str::FromStr;
use embassy_time::Instant;
use heapless::{String, Vec};

use crate::common::GlobalMutex;
use crate::common::error::{AppError, Result};
use crate::kernel::data::types::{DynamicValue, FieldMeta};
use crate::kernel::data::{DataSource, DataSourceCache};
use crate::kernel::driver::sensor::{DefaultSensorDriver, SensorDriver};
use crate::kernel::system::api::{DefaultSystemApi, NetworkClientApi};

// ======================== 常量定义（集中管理魔法值）========================
/// 天气API基础URL
const WEATHER_API_BASE_URL: &str = "https://api.weatherapi.com/v1/current.json";
/// 模拟API Key（建议后续从配置读取）
const DEFAULT_API_KEY: &str = "mock_api_key";
/// 数据源刷新间隔（秒）：2小时
const REFRESH_INTERVAL_SECS: u32 = 2 * 60 * 60;
/// 字段元数据最大容量
const MAX_FIELD_META: usize = 10;

// ======================== 类型别名（增强类型安全+可读性）========================
type String64 = String<64>;
type String128 = String<128>;
/// 天气传感器数据类型 (温度, 湿度)
type SensorWeatherData = (f32, f32);
/// API返回的天气数据类型 (温度, 湿度, 天气状况)
type ApiWeatherData = (f32, i32, String64);

// ======================== 结构体定义（规范+清晰）========================
/// 天气数据源结构体
pub struct WeatherDataSource {
    /// 系统API实例（全局互斥锁保护）
    system_api: &'static GlobalMutex<DefaultSystemApi>,
    /// 传感器驱动实例
    sensor_driver: DefaultSensorDriver,
    /// 地理位置ID（嵌入式堆字符串）
    location_id: String<10>,
    /// 数据源缓存
    cache: DataSourceCache,
    /// 字段元数据列表
    fields: Vec<FieldMeta, MAX_FIELD_META>,
}

impl WeatherDataSource {
    /// 创建新的天气数据源实例
    /// # 参数
    /// - system_api: 系统API全局实例
    /// - sensor_driver: 传感器驱动实例
    /// - location_id: 地理位置ID（如城市编码/经纬度）
    pub fn new(
        system_api: &'static GlobalMutex<DefaultSystemApi>,
        sensor_driver: DefaultSensorDriver,
    ) -> Result<Self> {
        // 初始化字段元数据
        let fields = Self::init_field_meta()?;

        // 初始化地理位置ID
        let location_id = String::from_str(location_id).map_err(|_| AppError::InvalidLocationId)?;

        Ok(Self {
            system_api,
            sensor_driver,
            location_id,
            cache: DataSourceCache::default(),
            fields,
        })
    }

    // ======================== 辅助函数（封装重复逻辑）========================
    /// 初始化字段元数据（统一管理，避免冗余）
    fn init_field_meta() -> Result<Vec<FieldMeta, MAX_FIELD_META>> {
        let mut fields = Vec::new();

        // 当前天气字段
        Self::push_field_meta(&mut fields, "weather.temperature", DynamicValue::Float(0.0))?;
        Self::push_field_meta(&mut fields, "weather.humidity", DynamicValue::Integer(0))?;
        Self::push_field_meta(
            &mut fields,
            "weather.condition",
            DynamicValue::String(String::new()),
        )?;
        Self::push_field_meta(&mut fields, "weather.wind_speed", DynamicValue::Float(0.0))?;
        Self::push_field_meta(&mut fields, "weather.visibility", DynamicValue::Integer(0))?;

        // 预报天气字段
        Self::push_field_meta(
            &mut fields,
            "weather.forecast.day1.hi_temp",
            DynamicValue::Float(0.0),
        )?;
        Self::push_field_meta(
            &mut fields,
            "weather.forecast.day2.hi_temp",
            DynamicValue::Float(0.0),
        )?;
        Self::push_field_meta(
            &mut fields,
            "weather.forecast.day3.hi_temp",
            DynamicValue::Float(0.0),
        )?;

        Ok(fields)
    }

    /// 安全添加字段元数据
    fn push_field_meta(
        fields: &mut Vec<FieldMeta, MAX_FIELD_META>,
        name: &str,
        content: DynamicValue,
    ) -> Result<()> {
        let field_name = String::from_str(name).map_err(|_| AppError::InvalidFieldName)?;

        fields
            .push(FieldMeta {
                name: field_name,
                content,
            })
            .map_err(|_| AppError::FieldMetaLimitExceeded)?;

        Ok(())
    }

    /// 安全设置缓存字段
    fn set_cache_field(&mut self, name: &str, value: DynamicValue) -> Result<()> {
        let field_name = String::from_str(name).map_err(|_| AppError::InvalidFieldName)?;

        self.cache
            .set_field(field_name, value)
            .map_err(|_| AppError::CacheSetFailed)?;

        Ok(())
    }

    /// 构建天气API请求URL
    fn build_api_url(&self, api_key: &str) -> Result<String128> {
        let url = format!(
            "{}?key={}&q={}&aqi=no",
            WEATHER_API_BASE_URL, api_key, self.location_id
        );
        String::from_str(&url).map_err(|_| AppError::InvalidApiUrl)
    }

    // ======================== 业务逻辑函数（单一职责）========================
    /// 从传感器获取本地天气数据（模拟实现，后续替换为真实驱动调用）
    async fn get_local_weather_data(&mut self) -> Result<SensorWeatherData> {
        let ret = self.sensor_driver.read().await.unwrap();
        Ok((ret.temperature, ret.humidity))
    }

    /// 从远程API获取天气数据
    async fn get_remote_weather_data(&self) -> Result<ApiWeatherData> {
        let url = self.build_api_url(DEFAULT_API_KEY)?;
        let response: [u8; 1024] = self.system_api.lock().await.http_get(&url).await?;

        // 模拟API响应解析（真实场景需解析JSON/XML）
        let (temp, humidity, condition) = self.parse_api_response(&response)?;
        Ok((temp, humidity, condition))
    }

    /// 解析API响应数据（模拟实现）
    fn parse_api_response(&self, _response: &[u8]) -> Result<ApiWeatherData> {
        // 真实场景：解析JSON响应，例如使用serde-json-core
        let temp = 22.5_f64;
        let humidity = 55_i64;
        let condition = String::from_str("晴天").map_err(|_| AppError::InvalidWeatherCondition)?;

        Ok((temp, humidity, condition))
    }

    /// 更新天气缓存（统一处理API/本地数据的缓存更新）
    fn update_weather_cache(
        &mut self,
        temp: f32,
        humidity: i32,
        condition: String64,
    ) -> Result<()> {
        self.set_cache_field("weather.temperature", DynamicValue::Float(temp))?;
        self.set_cache_field("weather.humidity", DynamicValue::Integer(humidity))?;
        self.set_cache_field("weather.condition", DynamicValue::String(condition))?;
        Ok(())
    }
}

// ======================== DataSource Trait 实现（规范+清晰）========================
#[async_trait(?Send)]
impl DataSource for WeatherDataSource {
    /// 获取数据源名称
    fn name(&self) -> &'static str {
        "weather"
    }

    /// 获取字段值
    fn get_field_value(&self, name: &str) -> Result<DynamicValue> {
        self.cache
            .get_field(name)
            .ok_or_else(|| AppError::FieldNotFound)
            .cloned()
    }

    /// 刷新数据源（核心逻辑：优先API，降级本地传感器）
    async fn refresh(&mut self, _system_api: &'static GlobalMutex<DefaultSystemApi>) -> Result<()> {
        // 优先从远程API获取数据，失败则降级到本地传感器
        let weather_data = match self.get_remote_weather_data().await {
            Ok(data) => {
                log::debug!("成功从远程API获取天气数据");
                data
            }
            Err(e) => {
                log::warn!("远程API获取天气数据失败（{}），使用本地传感器数据", e);
                let (temp, humidity) = self.get_local_weather_data().await?;
                (
                    temp,
                    humidity,
                    String::from_str("本地数据").map_err(|_| AppError::InvalidWeatherCondition)?,
                )
            }
        };

        // 更新缓存
        self.update_weather_cache(weather_data.0, weather_data.1, weather_data.2)?;

        // 更新缓存状态
        self.cache.valid = true;
        self.cache.last_updated = Instant::now();

        log::info!(
            "天气数据源刷新完成 | 温度: {:.1}℃ | 湿度: {}% | 状况: {}",
            weather_data.0,
            weather_data.1,
            weather_data.2
        );

        Ok(())
    }

    /// 获取刷新间隔（秒）
    fn refresh_interval(&self) -> u32 {
        REFRESH_INTERVAL_SECS
    }
}
