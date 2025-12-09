//! 天气数据源模块
//! 提供天气相关数据的数据源实现

use alloc::boxed::Box;
use alloc::format;
use async_trait::async_trait;
use core::str::FromStr;
use embassy_time::Instant;
use heapless::{String, Vec};

mod weather_types;

// 导入天气相关类型
use crate::kernel::data::sources::weather::weather_types::{
    DailyWeather, QWeatherResponse, QWeatherStatusCode, WindDirection,
};

use crate::common::GlobalMutex;
use crate::common::error::{AppError, Result};
use crate::kernel::data::types::{DynamicValue, FieldMeta};
use crate::kernel::data::{DataSource, DataSourceCache};
use crate::kernel::driver::sensor::{DefaultSensorDriver, SensorDriver};
use crate::kernel::system::api::{DefaultSystemApi, NetworkClientApi, SystemApi};

// ======================== 常量定义（集中管理魔法值）========================
/// 天气API基础URL（和风天气）
const WEATHER_API_BASE_URL: &str = "https://devapi.qweather.com/v7/weather/3d";
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
    pub async fn new(
        system_api: &'static GlobalMutex<DefaultSystemApi>,
        sensor_driver: DefaultSensorDriver,
    ) -> Result<Self> {
        // 初始化字段元数据
        let fields = Self::init_field_meta()?;

        // 从配置中获取地理位置ID
        let location_id = system_api
            .lock()
            .await
            .get_data_by_path("config.weather.location_id")
            .await?;

        // 将DynamicValue转换为String
        let location_id = match location_id {
            DynamicValue::String(s) => {
                // 将String<64>转换为String<10>
                heapless::String::<10>::from_str(&s).map_err(|_| AppError::InvalidLocationId)?
            },
            _ => return Err(AppError::InvalidLocationId),
        };

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
            "{}?key={}&location={}&lang=zh-hans&unit=m",
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
    async fn get_remote_weather_data(&self) -> Result<heapless::Vec<DailyWeather, 3>> {
        // 从配置中获取API密钥
        let api_key = self
            .system_api
            .lock()
            .await
            .get_data_by_path("config.weather.api_key")
            .await?;
        let api_key = match api_key {
            DynamicValue::String(s) => s,
            _ => return Err(AppError::InvalidApiKey),
        };

        let url = self.build_api_url(&api_key)?;
        let response = self.system_api.lock().await.https_get(&url).await?;

        // 解析API响应
        let weather_data = self.parse_api_response(&response)?;
        Ok(weather_data)
    }

    /// 解析API响应数据
    fn parse_api_response(&self, response: &str) -> Result<heapless::Vec<DailyWeather, 3>> {
        // 使用serde_json解析JSON响应
        let result = serde_json::from_str::<QWeatherResponse>(response)
            .map_err(|_| AppError::JsonParseFailed)?;

        // 检查响应状态码
        if result.code != QWeatherStatusCode::Success {
            return Err(AppError::WeatherApiError);
        }

        // 返回所有的天气预报数据（最多3天）
        if !result.daily.is_empty() {
            Ok(result.daily)
        } else {
            Err(AppError::WeatherDataNotFound)
        }
    }

    /// 更新天气缓存（统一处理API/本地数据的缓存更新）
    fn update_weather_cache(&mut self, daily_weather_list: &heapless::Vec<DailyWeather, 3>) -> Result<()> {
        // 如果有数据，先处理当天的天气数据
        if let Some(today) = daily_weather_list.get(0) {
            // 保存当天的最高温度
            self.set_cache_field(
                "weather.temperature",
                DynamicValue::Float(today.temp_max as f32),
            )?;
            // 保存湿度
            self.set_cache_field(
                "weather.humidity",
                DynamicValue::Integer(today.humidity as i32),
            )?;
            // 保存天气状况
            let text_day = heapless::String::<64>::from_str(&today.text_day).map_err(|_| AppError::InvalidWeatherCondition)?;
            self.set_cache_field(
                "weather.condition",
                DynamicValue::String(text_day),
            )?;
            // 保存风向
            self.set_cache_field(
                "weather.wind_direction",
                DynamicValue::Integer(today.wind_direction as i32),
            )?;
            // 保存风速
            self.set_cache_field(
                "weather.wind_speed",
                DynamicValue::Integer(today.wind_speed as i32),
            )?;
            // 保存降水量
            self.set_cache_field("weather.precip", DynamicValue::Float(today.precip))?;
            // 保存紫外线指数
            self.set_cache_field(
                "weather.uv_index",
                DynamicValue::Integer(today.uv_index as i32),
            )?;
            // 保存最低温度
            self.set_cache_field(
                "weather.temp_min",
                DynamicValue::Float(today.temp_min as f32),
            )?;
        }

        // 处理3天的天气预报数据
        for (index, daily) in daily_weather_list.iter().enumerate() {
            let day = index + 1;
            // 保存最高温度
            self.set_cache_field(
                &format!("weather.forecast.day{}.hi_temp", day),
                DynamicValue::Float(daily.temp_max as f32),
            )?;
            // 保存最低温度
            self.set_cache_field(
                &format!("weather.forecast.day{}.lo_temp", day),
                DynamicValue::Float(daily.temp_min as f32),
            )?;
            // 保存天气状况
            let text_day = heapless::String::<64>::from_str(&daily.text_day).map_err(|_| AppError::InvalidWeatherCondition)?;
            self.set_cache_field(
                &format!("weather.forecast.day{}.condition", day),
                DynamicValue::String(text_day),
            )?;
            // 保存日期
            let date = heapless::String::<64>::from_str(&daily.date).map_err(|_| AppError::InvalidDate)?;
            self.set_cache_field(
                &format!("weather.forecast.day{}.date", day),
                DynamicValue::String(date),
            )?;
        }

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
        let weather_data_list = match self.get_remote_weather_data().await {
            Ok(data) => {
                log::debug!("成功从远程API获取天气数据");
                data
            }
            Err(e) => {
                log::warn!("远程API获取天气数据失败（{}），使用本地传感器数据", e);
                let (temp, humidity) = self.get_local_weather_data().await?;

                // 创建本地传感器数据的DailyWeather结构
                let local_weather = DailyWeather {
                    date: heapless::String::from_str("").unwrap(), // 空日期
                    temp_max: temp as i8,                          // 假设传感器返回的是最高温度
                    temp_min: temp as i8,                          // 假设传感器返回的是最低温度
                    icon_day: heapless::String::from_str("").unwrap(), // 空图标
                    text_day: heapless::String::from_str("本地数据").unwrap(), // 本地数据标记
                    icon_night: Default::default(),                // 默认夜间图标
                    text_night: heapless::String::from_str("本地数据").unwrap(), // 本地数据标记
                    wind_direction: WindDirection::None,           // 无风向数据
                    wind_speed: 0,                                 // 无风速度数据
                    humidity: humidity as u8,                      // 湿度
                    precip: 0.0,                                   // 无降水量数据
                    uv_index: 0,                                   // 无紫外线数据
                };

                // 创建只包含本地数据的向量
                let mut list = heapless::Vec::<DailyWeather, 3>::new();
                list.push(local_weather).map_err(|_| AppError::DataCapacityExceeded)?;
                list
            }
        };

        // 更新缓存
        self.update_weather_cache(&weather_data_list)?;

        // 更新缓存状态
        self.cache.valid = true;
        self.cache.last_updated = Instant::now();

        // 记录日志
        if let Some(today) = weather_data_list.get(0) {
            log::info!(
                "天气数据源刷新完成 | 温度: {}-{}℃ | 湿度: {}% | 状况: {}",
                today.temp_min,
                today.temp_max,
                today.humidity,
                today.text_day
            );
        }

        Ok(())
    }

    /// 获取刷新间隔（秒）
    fn refresh_interval(&self) -> u32 {
        REFRESH_INTERVAL_SECS
    }
}