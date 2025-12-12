//! 天气数据源模块
//! 提供天气相关数据的数据源实现

use alloc::boxed::Box;
use async_trait::async_trait;
use core::str::FromStr;
use embassy_time::Duration;
use heapless::{String, format};

mod weather_types;

// 导入天气相关类型
use crate::kernel::data::sources::weather::weather_types::{
    DailyWeather, QWeatherResponse, QWeatherStatusCode,
};

use crate::common::error::{AppError, Result};
use crate::common::{GlobalMutex, GlobalRwLockWriteGuard};
use crate::kernel::data::DataSource;
use crate::kernel::data::DynamicValue;
use crate::kernel::data::types::{
    CacheKey, CacheKeyValueMap, CacheStringValue, HeaplessVec, KEY_LENGTH,
};
use crate::kernel::system::api::{DefaultSystemApi, HardwareApi, NetworkClientApi, SystemApi};

// ======================== 常量定义（集中管理魔法值）========================
/// 天气API基础URL（和风天气）
const WEATHER_API_BASE_URL: &str = "https://devapi.qweather.com/v7/weather/3d";
/// 数据源刷新间隔（秒）：2小时
const REFRESH_INTERVAL_SECS: u64 = 2 * 60 * 60;
/// 全局缓存key前缀
const CACHE_KEY_PREFIX: &str = "weather";
/// URL最大长度
const URL_LENGTH: usize = 128;
/// 传感器数值字符串长度（如 "25.5" 占5位）
const SENSOR_VALUE_LENGTH: usize = 8;

// ======================== 类型别名（增强类型安全+可读性）========================
type UrlString = String<URL_LENGTH>;
/// 天气传感器数据类型 (温度, 湿度)
type SensorWeatherData = (f32, f32);
/// 传感器数值字符串类型
type SensorValueString = String<SENSOR_VALUE_LENGTH>;

// ======================== 结构体定义（删除本地缓存，适配全局缓存）========================
/// 天气数据源结构体
pub struct WeatherDataSource {
    /// 系统API实例（全局互斥锁保护）
    system_api: &'static GlobalMutex<DefaultSystemApi>,
}

impl WeatherDataSource {
    /// 创建新的天气数据源实例
    /// # 参数
    /// - system_api: 系统API全局实例
    pub async fn new(system_api: &'static GlobalMutex<DefaultSystemApi>) -> Result<Self> {
        Ok(Self { system_api })
    }

    /// 构建全局缓存key（拼接前缀：weather.xxx）
    fn build_cache_key(&self, field: &str) -> Result<CacheKey> {
        let full_key = format!(KEY_LENGTH; "{}.{}", CACHE_KEY_PREFIX, field).map_err(|_| {
            log::error!("Weather cache key too long: {}", field);
            AppError::InvalidFieldName
        })?;
        Ok(full_key)
    }

    /// 安全写入全局缓存字段
    fn write_cache_field(
        &self,
        cache_guard: &mut GlobalRwLockWriteGuard<CacheKeyValueMap>,
        key: &str,
        value: DynamicValue,
    ) -> Result<()> {
        let cache_key = self.build_cache_key(key)?;
        cache_guard.insert(cache_key, value);
        Ok(())
    }

    /// 辅助方法：将f32数值转换为CacheStringValue（适配传感器数据）
    fn f32_to_cache_string(&self, value: f32) -> Result<CacheStringValue> {
        // 格式化保留1位小数，避免精度冗余
        let value_str = format!(SENSOR_VALUE_LENGTH; "{0:.1}", value).map_err(|_| {
            log::error!("Failed to format sensor value: {}", value);
            AppError::InvalidSensorValue
        })?;
        CacheStringValue::from_str(&value_str).map_err(|_| AppError::InvalidSensorValue)
    }

    // ======================== 业务逻辑函数（单一职责，无本地缓存依赖）========================
    /// 构建天气API请求URL
    async fn build_api_url(&self, api_key: &str) -> Result<UrlString> {
        // 从配置中获取地理位置ID（从全局缓存读取）
        let system_api_guard = self.system_api.lock().await;
        let location_id = system_api_guard
            .get_data_by_path("config.weather.location_id")
            .await?;
        drop(system_api_guard);

        // 将DynamicValue转换为String并构建URL
        let url = match location_id {
            DynamicValue::String(s) => format!(
                URL_LENGTH;
                "{}?key={}&location={}&lang=zh-hans&unit=m",
                WEATHER_API_BASE_URL, api_key, s
            ),
            _ => return Err(AppError::InvalidLocationId),
        }
        .map_err(|_| AppError::InvalidApiUrl)?;

        Ok(url)
    }

    /// 从传感器获取本地天气数据（独立逻辑，确保必执行）
    async fn get_local_weather_data(&self) -> Result<SensorWeatherData> {
        let system_api_guard = self.system_api.lock().await;
        let temperature = system_api_guard
            .get_hardware_api()
            .get_temperature()
            .await?;
        let humidity = system_api_guard.get_hardware_api().get_humidity().await?;
        drop(system_api_guard);

        Ok((temperature as f32, humidity as f32))
    }

    /// 从远程API获取天气数据
    async fn get_remote_weather_data(&self) -> Result<HeaplessVec<DailyWeather, 3>> {
        // 从配置中获取API密钥
        let system_api_guard = self.system_api.lock().await;
        let api_key = system_api_guard
            .get_data_by_path("config.weather.api_key")
            .await?;
        drop(system_api_guard);

        let api_key = match api_key {
            DynamicValue::String(s) => s,
            _ => return Err(AppError::InvalidApiKey),
        };

        // 构建并发起HTTP请求
        let url = self.build_api_url(&api_key).await?;
        let system_api_guard = self.system_api.lock().await;
        let response = system_api_guard
            .get_network_client_api()
            .https_get(&url)
            .await?;
        drop(system_api_guard);

        // 解析API响应
        let weather_data = self.parse_api_response(&response)?;
        Ok(weather_data)
    }

    /// 解析API响应数据
    fn parse_api_response(&self, response: &str) -> Result<HeaplessVec<DailyWeather, 3>> {
        let result = serde_json::from_str::<QWeatherResponse>(response)
            .map_err(|_| AppError::JsonParseFailed)?;

        // 检查响应状态码
        if result.code != QWeatherStatusCode::Success {
            log::error!("Weather API error: code={:?}", result.code);
            return Err(AppError::WeatherApiError);
        }

        // 返回有效数据或报错
        if !result.daily.is_empty() {
            Ok(result.daily)
        } else {
            log::warn!("Weather API returned empty daily data");
            Err(AppError::WeatherDataNotFound)
        }
    }

    /// 更新天气数据到全局缓存（包含新增字段）
    fn update_online_weather_cache(
        &self,
        cache_guard: &mut GlobalRwLockWriteGuard<CacheKeyValueMap>,
        daily_weather_list: &HeaplessVec<DailyWeather, 3>,
    ) -> Result<()> {
        // 4. 处理核心天气数据（原有逻辑）
        if let Some(today) = daily_weather_list.get(0) {
            // 核心天气字段
            self.write_cache_field(
                cache_guard,
                "temperature",
                DynamicValue::Float(today.temp_max as f32),
            )?;
            self.write_cache_field(
                cache_guard,
                "humidity",
                DynamicValue::Integer(today.humidity as i32),
            )?;
            self.write_cache_field(
                cache_guard,
                "condition",
                DynamicValue::String(
                    CacheStringValue::from_str(&today.text_day)
                        .map_err(|_| AppError::InvalidWeatherCondition)?,
                ),
            )?;
            self.write_cache_field(
                cache_guard,
                "wind_direction",
                DynamicValue::Integer(today.wind_direction as i32),
            )?;
            self.write_cache_field(
                cache_guard,
                "wind_speed",
                DynamicValue::Integer(today.wind_speed as i32),
            )?;
            self.write_cache_field(cache_guard, "precip", DynamicValue::Float(today.precip))?;
            self.write_cache_field(
                cache_guard,
                "uv_index",
                DynamicValue::Integer(today.uv_index as i32),
            )?;
            self.write_cache_field(
                cache_guard,
                "temp_min",
                DynamicValue::Float(today.temp_min as f32),
            )?;
        }

        // 5. 处理3天的天气预报数据
        for (index, daily) in daily_weather_list.iter().enumerate() {
            let day = index + 1;
            let day_prefix = format!(KEY_LENGTH; "forecast.day{}", day).unwrap();

            // 最高/最低温度
            self.write_cache_field(
                cache_guard,
                &format!(KEY_LENGTH; "{}.hi_temp", day_prefix).unwrap(),
                DynamicValue::Float(daily.temp_max as f32),
            )?;
            self.write_cache_field(
                cache_guard,
                &format!(KEY_LENGTH; "{}.lo_temp", day_prefix).unwrap(),
                DynamicValue::Float(daily.temp_min as f32),
            )?;

            // 天气状况&日期
            self.write_cache_field(
                cache_guard,
                &format!(KEY_LENGTH; "{}.condition", day_prefix).unwrap(),
                DynamicValue::String(
                    CacheStringValue::from_str(&daily.text_day)
                        .map_err(|_| AppError::InvalidWeatherCondition)?,
                ),
            )?;
            self.write_cache_field(
                cache_guard,
                &format!(KEY_LENGTH; "{}.date", day_prefix).unwrap(),
                DynamicValue::String(
                    CacheStringValue::from_str(&daily.date)
                        .map_err(|_| AppError::InvalidWeatherDate)?,
                ),
            )?;
        }

        Ok(())
    }
}

// ======================== DataSource Trait 实现（适配全局缓存）========================
#[async_trait(?Send)]
impl DataSource for WeatherDataSource {
    /// 获取数据源名称（用于全局缓存key前缀）
    fn name(&self) -> &'static str {
        CACHE_KEY_PREFIX
    }

    /// 获取刷新间隔（秒）
    fn refresh_interval(&self) -> Duration {
        Duration::from_secs(REFRESH_INTERVAL_SECS)
    }

    /// 核心逻辑：刷新数据并直写全局缓存（包含新增字段处理）
    async fn refresh_with_cache(
        &mut self,
        _system_api: &'static GlobalMutex<DefaultSystemApi>,
        cache_guard: &mut GlobalRwLockWriteGuard<CacheKeyValueMap>,
    ) -> Result<()> {
        // 步骤1：优先获取本地传感器数据（必执行，确保sensor字段有值）
        let sensor_data = match self.get_local_weather_data().await {
            Ok(data) => data,
            Err(e) => {
                log::warn!("获取本地传感器数据失败（{}），使用默认值", e);
                (25.0, 50.0) // 降级默认值
            }
        };

        let forecast_valid = if let Ok(weather_data_list) = self.get_remote_weather_data().await {
            log::debug!("成功从远程API获取天气数据");
            self.update_online_weather_cache(cache_guard, &weather_data_list)?;
            true
        } else {
            // 2. 写入新增字段：本地传感器数据（CacheStringValue类型）
            let (sensor_temp, sensor_hum) = sensor_data;
            self.write_cache_field(
                cache_guard,
                "sensor.temperature",
                DynamicValue::String(self.f32_to_cache_string(sensor_temp)?),
            )?;
            self.write_cache_field(
                cache_guard,
                "sensor.humidity",
                DynamicValue::String(self.f32_to_cache_string(sensor_hum)?),
            )?;
            false
        };

        self.write_cache_field(
            cache_guard,
            "forecast.valid",
            DynamicValue::Boolean(forecast_valid),
        )?;

        Ok(())
    }
}
