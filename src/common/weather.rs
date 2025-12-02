use serde::Deserialize;

// 和风天气API状态码枚举
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum QWeatherStatusCode {
    /// 成功
    Success,
    /// 临时错误（可重试）
    TemporaryError,
    /// 永久错误（不应重试）
    PermanentError,
}

// 为 QWeatherStatusCode 实现自定义反序列化
impl<'de> serde::Deserialize<'de> for QWeatherStatusCode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = heapless::String::<3>::deserialize(deserializer)?;

        // 根据状态码判断类型
        match s.as_str() {
            "200" => Ok(QWeatherStatusCode::Success),
            // 临时错误类型：429（请求过多）、5xx（服务器错误）
            "429" => {
                log::warn!("和风天气API临时错误: 请求过多 (429)");
                Ok(QWeatherStatusCode::TemporaryError)
            }
            // 其他错误码
            code => {
                log::error!("和风天气API错误: 状态码 {}", code);
                Ok(QWeatherStatusCode::PermanentError)
            }
        }
    }
}

// 和风天气API响应根结构
// https://dev.qweather.com/docs/api/weather/weather-daily-forecast/
#[derive(Debug, Deserialize)]
pub struct QWeatherResponse {
    /// 状态码
    pub code: QWeatherStatusCode,
    /// API更新时间（yyyy-MM-ddTHH:mm+08:00）
    #[serde(rename = "updateTime")]
    pub update_time: heapless::String<20>,
    /// 3天预报数据
    pub daily: heapless::Vec<DailyWeather, 3>,
}

// 对外暴露的天气数据结构
#[derive(Debug, Clone, Default)]
pub struct WeatherData {
    /// 地区ID
    pub location_id: heapless::String<20>,
    /// 数据更新时间
    pub update_time: heapless::String<20>,
    /// 3天预报数据
    pub daily_forecast: heapless::Vec<DailyWeather, 3>,
}

// 风向枚举 - 适配和风天气API
// https://dev.qweather.com/docs/resource/wind-info/#wind-direction
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WindDirection {
    North,          // 北风 N 0 348.75-11.25
    NorthNortheast, // 东北偏北风 NNE 22.5 11.25-33.75
    Northeast,      // 东北风 NE 45 33.75-56.25
    EastNortheast,  // 东北偏东风 ENE 67.5 56.25-78.75
    East,           // 东风 E 90 78.75-101.25
    EastSoutheast,  // 东南偏东风 ESE 112.5 101.25-123.75
    Southeast,      // 东南风 SE 135 123.75-146.25
    SouthSoutheast, // 东南偏南风 SSE 157.5 146.25-168.75
    South,          // 南风 S 180 168.75-191.25
    SouthSouthwest, // 西南偏南风 SSW 202.5 191.25-213.75
    Southwest,      // 西南风 SW 225 213.75-236.25
    WestSouthwest,  // 西南偏西风 WSW 247.5 236.25-258.75
    West,           // 西风 W 270 258.75-281.25
    WestNorthwest,  // 西北偏西风 WNW 292.5 281.25-303.75
    Northwest,      // 西北风 NW 315 303.75-326.25
    NorthNorthwest, // 西北偏北风 NNW 337.5 326.25-348.75
    Rotational,     // 旋转风 Rotational -999 -
    None,           // 无持续风向 None -1 -
    Unknown,        // 未知风向
}

// 为 WindDirection 实现自定义反序列化
impl<'de> serde::Deserialize<'de> for WindDirection {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = heapless::String::<10>::deserialize(deserializer)?;

        // 根据输入字符串匹配对应的风向
        match s.as_str() {
            // 中文风向名称匹配
            "北风" => Ok(WindDirection::North),
            "东北偏北风" => Ok(WindDirection::NorthNortheast),
            "东北风" => Ok(WindDirection::Northeast),
            "东北偏东风" => Ok(WindDirection::EastNortheast),
            "东风" => Ok(WindDirection::East),
            "东南偏东风" => Ok(WindDirection::EastSoutheast),
            "东南风" => Ok(WindDirection::Southeast),
            "东南偏南风" => Ok(WindDirection::SouthSoutheast),
            "南风" => Ok(WindDirection::South),
            "西南偏南风" => Ok(WindDirection::SouthSouthwest),
            "西南风" => Ok(WindDirection::Southwest),
            "西南偏西风" => Ok(WindDirection::WestSouthwest),
            "西风" => Ok(WindDirection::West),
            "西北偏西风" => Ok(WindDirection::WestNorthwest),
            "西北风" => Ok(WindDirection::Northwest),
            "西北偏北风" => Ok(WindDirection::NorthNorthwest),
            "旋转风" => Ok(WindDirection::Rotational),
            "无持续风向" => Ok(WindDirection::None),

            // 英文风向代码匹配
            "N" => Ok(WindDirection::North),
            "NNE" => Ok(WindDirection::NorthNortheast),
            "NE" => Ok(WindDirection::Northeast),
            "ENE" => Ok(WindDirection::EastNortheast),
            "E" => Ok(WindDirection::East),
            "ESE" => Ok(WindDirection::EastSoutheast),
            "SE" => Ok(WindDirection::Southeast),
            "SSE" => Ok(WindDirection::SouthSoutheast),
            "S" => Ok(WindDirection::South),
            "SSW" => Ok(WindDirection::SouthSouthwest),
            "SW" => Ok(WindDirection::Southwest),
            "WSW" => Ok(WindDirection::WestSouthwest),
            "W" => Ok(WindDirection::West),
            "WNW" => Ok(WindDirection::WestNorthwest),
            "NW" => Ok(WindDirection::Northwest),
            "NNW" => Ok(WindDirection::NorthNorthwest),
            "Rotational" => Ok(WindDirection::Rotational),
            "None" => Ok(WindDirection::None),

            // 其他情况视为未知风向
            _ => Ok(WindDirection::Unknown),
        }
    }
}

// 天气图标枚举 - 适配和风天气API
// https://dev.qweather.com/docs/resource/icons/
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WeatherIcon {
    // 晴天
    ClearDay,   // 100 晴（白天）
    ClearNight, // 150 晴（夜间）

    // 多云
    CloudyDay,         // 101 多云（白天）
    CloudyNight,       // 151 多云（夜间）
    FewCloudsDay,      // 102 少云（白天）
    FewCloudsNight,    // 152 少云（夜间）
    PartlyCloudyDay,   // 103 晴间多云（白天）
    PartlyCloudyNight, // 153 晴间多云（夜间）
    Overcast,          // 104 阴（白天/夜间）

    // 雨
    ShowerDay,               // 300 阵雨（白天）
    ShowerNight,             // 350 阵雨（夜间）
    HeavyShowerDay,          // 301 强阵雨（白天）
    HeavyShowerNight,        // 351 强阵雨（夜间）
    Thunderstorm,            // 302 雷阵雨（白天/夜间）
    HeavyThunderstorm,       // 303 强雷阵雨（白天/夜间）
    ThunderstormWithHail,    // 304 雷阵雨伴有冰雹（白天/夜间）
    LightRain,               // 305 小雨（白天/夜间）
    ModerateRain,            // 306 中雨（白天/夜间）
    HeavyRain,               // 307 大雨（白天/夜间）
    ExtremeRain,             // 308 极端降雨（白天/夜间）
    Drizzle,                 // 309 毛毛雨/细雨（白天/夜间）
    Storm,                   // 310 暴雨（白天/夜间）
    HeavyStorm,              // 311 大暴雨（白天/夜间）
    SevereStorm,             // 312 特大暴雨（白天/夜间）
    FreezingRain,            // 313 冻雨（白天/夜间）
    LightToModerateRain,     // 314 小到中雨（白天/夜间）
    ModerateToHeavyRain,     // 315 中到大雨（白天/夜间）
    HeavyToStorm,            // 316 大到暴雨（白天/夜间）
    StormToHeavyStorm,       // 317 暴雨到大暴雨（白天/夜间）
    HeavyStormToSevereStorm, // 318 大暴雨到特大暴雨（白天/夜间）
    Rain,                    // 399 雨（白天/夜间）

    // 雪
    LightSnow,           // 400 小雪（白天/夜间）
    ModerateSnow,        // 401 中雪（白天/夜间）
    HeavySnow,           // 402 大雪（白天/夜间）
    SnowStorm,           // 403 暴雪（白天/夜间）
    Sleet,               // 404 雨夹雪（白天/夜间）
    RainAndSnow,         // 405 雨雪天气（白天/夜间）
    ShowerWithSnowDay,   // 406 阵雨夹雪（白天）
    ShowerWithSnowNight, // 456 阵雨夹雪（夜间）
    SnowShowerDay,       // 407 阵雪（白天）
    SnowShowerNight,     // 457 阵雪（夜间）
    LightToModerateSnow, // 408 小到中雪（白天/夜间）
    ModerateToHeavySnow, // 409 中到大雪（白天/夜间）
    HeavySnowToStorm,    // 410 大到暴雪（白天/夜间）
    Snow,                // 499 雪（白天/夜间）

    // 雾/霾/沙尘
    Mist,              // 500 薄雾（白天/夜间）
    Fog,               // 501 雾（白天/夜间）
    Haze,              // 502 霾（白天/夜间）
    DustBlowing,       // 503 扬沙（白天/夜间）
    Dust,              // 504 浮尘（白天/夜间）
    Sandstorm,         // 507 沙尘暴（白天/夜间）
    SevereSandstorm,   // 508 强沙尘暴（白天/夜间）
    DenseFog,          // 509 浓雾（白天/夜间）
    VeryDenseFog,      // 510 强浓雾（白天/夜间）
    ModerateHaze,      // 511 中度霾（白天/夜间）
    HeavyHaze,         // 512 重度霾（白天/夜间）
    SevereHaze,        // 513 严重霾（白天/夜间）
    ThickFog,          // 514 大雾（白天/夜间）
    ExtremelyDenseFog, // 515 特强浓雾（白天/夜间）

    // 其他
    Hot,  // 900 热（白天/夜间）
    Cold, // 901 冷（白天/夜间）

    // 未知图标
    Unknown, // 未知图标
}

// 为 WeatherIcon 实现自定义反序列化
impl<'de> serde::Deserialize<'de> for WeatherIcon {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = heapless::String::<3>::deserialize(deserializer)?;

        // 根据图标代码匹配对应的天气图标
        match s.as_str() {
            "100" => Ok(WeatherIcon::ClearDay),
            "150" => Ok(WeatherIcon::ClearNight),
            "101" => Ok(WeatherIcon::CloudyDay),
            "151" => Ok(WeatherIcon::CloudyNight),
            "102" => Ok(WeatherIcon::FewCloudsDay),
            "152" => Ok(WeatherIcon::FewCloudsNight),
            "103" => Ok(WeatherIcon::PartlyCloudyDay),
            "153" => Ok(WeatherIcon::PartlyCloudyNight),
            "104" => Ok(WeatherIcon::Overcast),
            "300" => Ok(WeatherIcon::ShowerDay),
            "350" => Ok(WeatherIcon::ShowerNight),
            "301" => Ok(WeatherIcon::HeavyShowerDay),
            "351" => Ok(WeatherIcon::HeavyShowerNight),
            "302" => Ok(WeatherIcon::Thunderstorm),
            "303" => Ok(WeatherIcon::HeavyThunderstorm),
            "304" => Ok(WeatherIcon::ThunderstormWithHail),
            "305" => Ok(WeatherIcon::LightRain),
            "306" => Ok(WeatherIcon::ModerateRain),
            "307" => Ok(WeatherIcon::HeavyRain),
            "308" => Ok(WeatherIcon::ExtremeRain),
            "309" => Ok(WeatherIcon::Drizzle),
            "310" => Ok(WeatherIcon::Storm),
            "311" => Ok(WeatherIcon::HeavyStorm),
            "312" => Ok(WeatherIcon::SevereStorm),
            "313" => Ok(WeatherIcon::FreezingRain),
            "314" => Ok(WeatherIcon::LightToModerateRain),
            "315" => Ok(WeatherIcon::ModerateToHeavyRain),
            "316" => Ok(WeatherIcon::HeavyToStorm),
            "317" => Ok(WeatherIcon::StormToHeavyStorm),
            "318" => Ok(WeatherIcon::HeavyStormToSevereStorm),
            "399" => Ok(WeatherIcon::Rain),
            "400" => Ok(WeatherIcon::LightSnow),
            "401" => Ok(WeatherIcon::ModerateSnow),
            "402" => Ok(WeatherIcon::HeavySnow),
            "403" => Ok(WeatherIcon::SnowStorm),
            "404" => Ok(WeatherIcon::Sleet),
            "405" => Ok(WeatherIcon::RainAndSnow),
            "406" => Ok(WeatherIcon::ShowerWithSnowDay),
            "456" => Ok(WeatherIcon::ShowerWithSnowNight),
            "407" => Ok(WeatherIcon::SnowShowerDay),
            "457" => Ok(WeatherIcon::SnowShowerNight),
            "408" => Ok(WeatherIcon::LightToModerateSnow),
            "409" => Ok(WeatherIcon::ModerateToHeavySnow),
            "410" => Ok(WeatherIcon::HeavySnowToStorm),
            "499" => Ok(WeatherIcon::Snow),
            "500" => Ok(WeatherIcon::Mist),
            "501" => Ok(WeatherIcon::Fog),
            "502" => Ok(WeatherIcon::Haze),
            "503" => Ok(WeatherIcon::DustBlowing),
            "504" => Ok(WeatherIcon::Dust),
            "507" => Ok(WeatherIcon::Sandstorm),
            "508" => Ok(WeatherIcon::SevereSandstorm),
            "509" => Ok(WeatherIcon::DenseFog),
            "510" => Ok(WeatherIcon::VeryDenseFog),
            "511" => Ok(WeatherIcon::ModerateHaze),
            "512" => Ok(WeatherIcon::HeavyHaze),
            "513" => Ok(WeatherIcon::SevereHaze),
            "514" => Ok(WeatherIcon::ThickFog),
            "515" => Ok(WeatherIcon::ExtremelyDenseFog),
            "900" => Ok(WeatherIcon::Hot),
            "901" => Ok(WeatherIcon::Cold),
            _ => Ok(WeatherIcon::Unknown),
        }
    }
}

// 为 WeatherIcon 实现 Default 特性
impl Default for WeatherIcon {
    fn default() -> Self {
        WeatherIcon::Unknown
    }
}

// 单天天气预报
#[derive(Debug, Clone, Deserialize)]
pub struct DailyWeather {
    /// 预报日期（yyyy-MM-dd）
    #[serde(rename = "fxDate")]
    pub date: heapless::String<20>,
    /// 最高温度（℃）
    #[serde(rename = "tempMax")]
    pub temp_max: i8,
    /// 最低温度（℃）
    #[serde(rename = "tempMin")]
    pub temp_min: i8,
    /// 白天天气图标
    #[serde(rename = "iconDay")]
    pub icon_day: WeatherIcon,
    /// 白天天气描述
    #[serde(rename = "textDay")]
    pub text_day: heapless::String<20>,
    /// 夜间天气图标
    #[serde(rename = "iconNight")]
    pub icon_night: WeatherIcon,
    /// 夜间天气描述
    #[serde(rename = "textNight")]
    pub text_night: heapless::String<20>,
    /// 白天风向
    #[serde(rename = "windDirDay")]
    pub wind_direction: WindDirection,
    /// 白天风速（km/h）
    #[serde(rename = "windSpeedDay")]
    pub wind_speed: u8,
    /// 相对湿度（%）
    pub humidity: u8,
    /// 降水量（mm）
    pub precip: f32,
    /// 紫外线指数
    #[serde(rename = "uvIndex")]
    pub uv_index: u8,
}

// 辅助函数：从字符串反序列化数字
fn deserialize_number_from_string<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
    T: core::str::FromStr,
{
    let s = heapless::String::<10>::deserialize(deserializer)?;
    s.parse::<T>()
        .map_err(|_| serde::de::Error::custom("Failed to parse number"))
}

// 辅助函数：从字符串反序列化浮点数
fn deserialize_float_from_string<'de, D>(deserializer: D) -> Result<f32, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = heapless::String::<10>::deserialize(deserializer)?;
    s.parse::<f32>()
        .map_err(|_| serde::de::Error::custom("Failed to parse float"))
}

// 辅助函数：从字符串反序列化可选数字
fn deserialize_optional_number_from_string<'de, D, T>(
    deserializer: D,
) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: core::str::FromStr,
{
    let s = Option::<heapless::String<10>>::deserialize(deserializer)?;
    match s {
        Some(s) if !s.is_empty() => s
            .parse::<T>()
            .map_err(|_| serde::de::Error::custom("Failed to parse number"))
            .map(Some),
        _ => Ok(None),
    }
}
