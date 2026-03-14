use super::api::WeatherDailyResponse;
use crate::types::SystemResult;
use crate::types::weather::{ForecastDay, WeatherCondition, WeatherInfo};
use alloc::vec::Vec;
use heapless::String;

#[allow(dead_code)]
type HeaplessStr8 = heapless::String<8>;
type HeaplessString8 = heapless::String<8>;

pub fn convert_daily_response(
    response: &WeatherDailyResponse,
    location: &str,
) -> SystemResult<WeatherInfo> {
    if response.code != "200" {
        return Err(crate::types::SystemError::NetworkError(
            crate::types::NetworkError::Unknown,
        ));
    }

    let daily = &response.daily;
    if daily.is_empty() {
        return Err(crate::types::SystemError::NetworkError(
            crate::types::NetworkError::Unknown,
        ));
    }

    let location: String<32> =
        String::try_from(location).unwrap_or_else(|_| String::try_from("未知").unwrap());

    let mut forecast = heapless::Vec::new();
    for day in daily.iter().take(3) {
        let high_temp = day.temp_max.parse::<i16>().unwrap_or(0);
        let low_temp = day.temp_min.parse::<i16>().unwrap_or(0);
        let humidity = parse_humidity(&day.humidity);
        let condition = parse_weather_condition(day.text_day.as_str());

        forecast
            .push(ForecastDay {
                date: parse_date(day.fx_date.as_str()),
                high_temp,
                low_temp,
                condition,
                humidity,
            })
            .ok();
    }

    let today = &daily[0];
    let current_temp = (today.temp_max.parse::<i16>().unwrap_or(0)
        + today.temp_min.parse::<i16>().unwrap_or(0))
        / 2;
    let current_humidity = parse_humidity(&today.humidity);

    Ok(WeatherInfo {
        location,
        current: crate::types::weather::CurrentWeather {
            temp: current_temp,
            feels_like: current_temp,
            humidity: current_humidity,
            condition: parse_weather_condition(today.text_day.as_str()),
            wind_speed: parse_u8(&today.wind_speed_day),
            wind_direction: parse_u16(&today.wind_360_day),
            visibility: parse_u16(&today.vis),
            pressure: parse_u16(&today.pressure),
            update_time: embassy_time::Instant::now().elapsed().as_secs() as i64,
        },
        forecast,
        last_update: embassy_time::Instant::now().elapsed().as_secs() as i64,
    })
}

fn parse_date(date_str: &str) -> i64 {
    let parts: Vec<&str> = date_str.split('-').collect();
    if parts.len() != 3 {
        return 0;
    }

    let year: i32 = parts[0].parse().unwrap_or(0);
    let month: u32 = parts[1].parse().unwrap_or(1);
    let day: u32 = parts[2].parse().unwrap_or(1);

    let days_from_epoch =
        (year - 1970) * 365 + days_before_year(year) + days_before_month(month) + (day as i32 - 1);
    days_from_epoch as i64 * 86400
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

fn parse_weather_condition(text: &str) -> WeatherCondition {
    match text {
        "晴" | "Sunny" | "Clear" => WeatherCondition::Sunny,
        "多云" | "Cloudy" => WeatherCondition::Cloudy,
        "阴" | "Overcast" => WeatherCondition::Overcast,
        "小雨" | "Light Rain" => WeatherCondition::LightRain,
        "中雨" | "Moderate Rain" => WeatherCondition::ModerateRain,
        "大雨" | "Heavy Rain" => WeatherCondition::HeavyRain,
        "雷暴" | "Thunderstorm" => WeatherCondition::Thunderstorm,
        "雪" | "Snow" => WeatherCondition::Snow,
        "雾" | "Fog" => WeatherCondition::Fog,
        "霾" | "Haze" | "Smog" => WeatherCondition::Haze,
        _ => WeatherCondition::Cloudy,
    }
}

fn parse_u8(s: &Option<HeaplessString8>) -> u8 {
    match s {
        Some(s) => {
            let slice = s.as_str();
            slice.parse::<u8>().unwrap_or(0)
        }
        None => 0,
    }
}

fn parse_u16(s: &Option<HeaplessString8>) -> u16 {
    match s {
        Some(s) => {
            let slice = s.as_str();
            slice.parse::<u16>().unwrap_or(0)
        }
        None => 0,
    }
}

fn parse_humidity(s: &Option<HeaplessString8>) -> u8 {
    match s {
        Some(h) => {
            let slice = h.as_str();
            slice.parse::<u8>().unwrap_or(50)
        }
        None => 50,
    }
}
