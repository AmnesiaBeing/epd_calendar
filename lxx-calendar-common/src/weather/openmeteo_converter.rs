use super::openmeteo::OpenMeteoResponse;
use crate::types::weather::{CurrentWeather, ForecastDay, WeatherCondition, WeatherInfo};
use heapless::String;

pub fn convert_openmeteo_response(response: &OpenMeteoResponse, location: &str) -> WeatherInfo {
    let condition = convert_weather_code_to_condition(response.current.weather_code);

    let current = CurrentWeather {
        temp: (response.current.temperature_2m * 10.0) as i16,
        feels_like: (response.current.apparent_temperature * 10.0) as i16,
        humidity: response.current.relative_humidity_2m as u8,
        condition,
        wind_speed: response.current.wind_speed_10m as u8,
        wind_direction: response.current.wind_direction_10m as u16,
        visibility: 10, // Open-Meteo doesn't provide visibility, use default
        pressure: 1013, // Open-Meteo doesn't provide pressure in basic API, use default
        update_time: embassy_time::Instant::now().elapsed().as_secs() as i64,
    };

    let mut forecast = heapless::Vec::new();
    let daily_count = response.daily.time.len().min(3);

    for i in 0..daily_count {
        let condition = convert_weather_code_to_condition(response.daily.weather_code[i]);
        let date_str = response.daily.time[i].as_str();
        let date = parse_date_from_iso(date_str);

        forecast
            .push(ForecastDay {
                date,
                high_temp: (response.daily.temperature_2m_max[i] * 10.0) as i16,
                low_temp: (response.daily.temperature_2m_min[i] * 10.0) as i16,
                condition,
                humidity: 50, // Open-Meteo doesn't provide daily humidity in basic API
            })
            .ok();
    }

    WeatherInfo {
        location: String::try_from(location).unwrap_or_else(|_| String::try_from("未知").unwrap()),
        current,
        forecast,
        last_update: embassy_time::Instant::now().elapsed().as_secs() as i64,
    }
}

fn parse_date_from_iso(date_str: &str) -> i64 {
    // Parse date from "YYYY-MM-DD" format to Unix timestamp
    if let Some((year_part, rest)) = date_str.split_once('-') {
        if let Some((month_part, day_part)) = rest.split_once('-') {
            let year: i32 = year_part.parse().unwrap_or(0);
            let month: u32 = month_part.parse().unwrap_or(1);
            let day: u32 = day_part.parse().unwrap_or(1);

            let days_from_epoch = (year - 1970) * 365
                + days_before_year(year)
                + days_before_month(month)
                + (day as i32 - 1);
            return days_from_epoch as i64 * 86400;
        }
    }
    0
}

fn days_before_year(year: i32) -> i32 {
    let mut days = 0;
    for y in 1970..year {
        if is_leap_year(y) {
            days += 366;
        } else {
            days += 365;
        }
    }
    days
}

fn days_before_month(month: u32) -> i32 {
    const DAYS_BEFORE_MONTH: [i32; 12] = [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334];
    DAYS_BEFORE_MONTH[(month - 1) as usize]
}

fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

fn convert_weather_code_to_condition(code: u8) -> WeatherCondition {
    match code {
        0 => WeatherCondition::Sunny,
        1 => WeatherCondition::Sunny,
        2 => WeatherCondition::Cloudy,
        3 => WeatherCondition::Overcast,
        45 | 48 => WeatherCondition::Fog,
        51 | 53 | 55 => WeatherCondition::LightRain,
        61 | 63 | 65 => WeatherCondition::ModerateRain,
        71 | 73 | 75 => WeatherCondition::Snow,
        80 | 81 | 82 => WeatherCondition::LightRain,
        95 | 96 | 99 => WeatherCondition::Thunderstorm,
        _ => WeatherCondition::Cloudy,
    }
}

pub fn get_weather_description(code: u8) -> &'static str {
    match code {
        0 => "晴",
        1 => "晴",
        2 => "多云",
        3 => "阴",
        45 | 48 => "雾",
        51 | 53 | 55 => "小雨",
        61 | 63 | 65 => "中雨",
        71 | 73 | 75 => "雪",
        80 | 81 | 82 => "阵雨",
        95 | 96 | 99 => "雷暴",
        _ => "多云",
    }
}

/// 根据 WMO 天气代码和昼夜状态获取和风图标代码
/// 
/// # 参数
/// - `code`: WMO 天气代码 (0-99)
/// - `is_day`: 是否为白天 (true=白天，false=夜间)
/// 
/// # 返回
/// 和风天气图标代码字符串（如 "100", "150" 等）
pub fn get_icon_code(code: u8, is_day: bool) -> &'static str {
    match (code, is_day) {
        // 晴空
        (0, true) => "100",
        (0, false) => "150",
        // 主要晴朗/部分多云
        (1 | 2, true) => "102",
        (1 | 2, false) => "151",
        // 阴天
        (3, _) => "104",
        // 雾
        (45 | 48, _) => "501",
        // 毛毛雨/小雨
        (51 | 53 | 55 | 56 | 57, _) => "309",
        // 雨/冻雨
        (61 | 63 | 65 | 66 | 67, _) => "306",
        // 雪/雪粒
        (71 | 73 | 75 | 77, _) => "400",
        // 阵雨
        (80 | 81 | 82, _) => "300",
        // 阵雪
        (85 | 86, _) => "406",
        // 雷暴（可能伴冰雹）
        (95 | 96 | 99, _) => "302",
        // 默认
        _ => "104",
    }
}
