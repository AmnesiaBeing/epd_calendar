//! 图标绘制

use super::buffer::{Color, FrameBuffer};

/// 天气类型定义
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeatherType {
    Sunny,
    Cloudy,
    Rainy,
    Thunder,
    Snowy,
    Foggy,
}

impl WeatherType {
    /// 获取天气类型数量
    pub fn count() -> usize {
        6
    }
}

/// 64x64图标占位数据（实际使用时替换为完整数据）
const SUNNY_ICON: [u8; 512] = [0; 512];
const CLOUDY_ICON: [u8; 512] = [0; 512];
const RAINY_ICON: [u8; 512] = [0; 512];
const THUNDER_ICON: [u8; 512] = [0; 512];
const SNOWY_ICON: [u8; 512] = [0; 512];
const FOGGY_ICON: [u8; 512] = [0; 512];

/// 绘制天气图标
pub fn draw_weather_icon(
    buffer: &mut FrameBuffer,
    x: usize,
    y: usize,
    weather_type: WeatherType,
    color: Color,
) {
    let icon_data = match weather_type {
        WeatherType::Sunny => &SUNNY_ICON,
        WeatherType::Cloudy => &CLOUDY_ICON,
        WeatherType::Rainy => &RAINY_ICON,
        WeatherType::Thunder => &THUNDER_ICON,
        WeatherType::Snowy => &SNOWY_ICON,
        WeatherType::Foggy => &FOGGY_ICON,
    };

    const ICON_SIZE: usize = 64;

    // 绘制图标到缓冲区
    for row in 0..ICON_SIZE {
        for col in 0..ICON_SIZE {
            // 计算当前像素在图标数据中的位置
            let byte_index = (row * ICON_SIZE + col) / 8;
            let bit_index = 7 - (col % 8);

            // 检查该像素是否需要绘制
            if icon_data[byte_index] & (1 << bit_index) != 0 {
                buffer.set_pixel(x + col, y + row, color);
            }
        }
    }
}
