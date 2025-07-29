//! 天气显示功能

use super::super::graphics::{
    buffer::Color,
    icon::{WeatherType, draw_weather_icon},
    text::draw_string,
};
use freetype::face::Face;

/// 天气信息
pub struct WeatherInfo {
    pub weather_type: WeatherType,
    pub temperature: i32, // 温度(°C)
    pub humidity: u8,     // 湿度(%)
    pub pressure: f32,    // 气压(kPa)
}

impl Default for WeatherInfo {
    fn default() -> Self {
        Self {
            weather_type: WeatherType::Sunny,
            temperature: 25,
            humidity: 60,
            pressure: 101.3,
        }
    }
}

/// 绘制天气信息到缓冲区
pub fn draw_weather(
    buffer: &mut super::super::graphics::buffer::FrameBuffer,
    face: &mut Face,
    info: &WeatherInfo,
) -> Result<(), freetype::Error> {
    // 绘制天气图标
    draw_weather_icon(buffer, 600, 80, info.weather_type, Color::Yellow);

    // 绘制天气文字描述
    let weather_text = match info.weather_type {
        WeatherType::Sunny => "晴天",
        WeatherType::Cloudy => "多云",
        WeatherType::Rainy => "雨天",
        WeatherType::Thunder => "雷暴",
        WeatherType::Snowy => "雪天",
        WeatherType::Foggy => "雾天",
    };
    draw_string(buffer, face, weather_text, 620, 220, Color::Black, 30)?;

    // 绘制温湿度、气压
    let temp_str = format!("温度:{}°C", info.temperature);
    draw_string(buffer, face, &temp_str, 40, 280, Color::Black, 24)?;

    let humidity_str = format!("湿度:{}%", info.humidity);
    draw_string(buffer, face, &humidity_str, 40, 310, Color::Black, 24)?;

    let pressure_str = format!("气压:{:.1}kPa", info.pressure);
    draw_string(buffer, face, &pressure_str, 40, 340, Color::Black, 24)?;

    Ok(())
}
