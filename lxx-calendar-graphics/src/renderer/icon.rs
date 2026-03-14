//! 图标渲染模块
//! 负责渲染各种图标（时间、天气、电池等）

extern crate alloc;

use super::framebuffer::{Color, Framebuffer};
use crate::assets::generated_icons::{IconId, WeatherIcon};
use lxx_calendar_common::SystemResult;
use lxx_calendar_common::types::weather::WeatherCondition;

/// 图标渲染器
pub struct IconRenderer;

impl IconRenderer {
    pub fn new() -> Self {
        Self
    }

    /// 渲染天气图标（使用 WeatherCondition）
    /// 参数：
    /// - framebuffer: 渲染缓冲区
    /// - x: X 坐标
    /// - y: Y 坐标
    /// - condition: 天气条件
    /// - is_day: 是否为白天
    pub fn render_weather_icon<const SIZE: usize>(
        &self,
        framebuffer: &mut Framebuffer<SIZE>,
        x: u16,
        y: u16,
        condition: WeatherCondition,
        is_day: bool,
    ) -> SystemResult<()> {
        // 将 WeatherCondition 转换为 WeatherIcon
        let icon = self.condition_to_weather_icon(condition, is_day);
        self.render_weather_icon_by_enum(framebuffer, x, y, icon)
    }

    /// 根据天气代码渲染图标
    /// 参数：
    /// - framebuffer: 渲染缓冲区
    /// - x: X 坐标
    /// - y: Y 坐标
    /// - icon_code: 和风天气图标代码（如 "100", "300" 等）
    pub fn render_weather_icon_by_code<const SIZE: usize>(
        &self,
        framebuffer: &mut Framebuffer<SIZE>,
        x: u16,
        y: u16,
        icon_code: &str,
    ) -> SystemResult<()> {
        if let Some(icon) = WeatherIcon::from_api_str(icon_code) {
            self.render_weather_icon_by_enum(framebuffer, x, y, icon)
        } else {
            // 如果图标代码无效，使用默认图标（阴天）
            self.render_weather_icon_by_enum(framebuffer, x, y, WeatherIcon::Icon104)
        }
    }

    /// 渲染天气图标（使用 WeatherIcon 枚举）
    fn render_weather_icon_by_enum<const SIZE: usize>(
        &self,
        framebuffer: &mut Framebuffer<SIZE>,
        x: u16,
        y: u16,
        icon: WeatherIcon,
    ) -> SystemResult<()> {
        let icon_id = IconId::Weather(icon);
        let bitmap_data = icon_id.data();
        let width = icon_id.width();
        let height = icon_id.height();

        // 从位图数据渲染图标
        self.render_bitmap(framebuffer, x, y, bitmap_data, width, height)
    }

    /// 从位图数据渲染图标
    /// 位图格式：单色位图，每像素 1 位，0=黑色，1=白色
    fn render_bitmap<const SIZE: usize>(
        &self,
        framebuffer: &mut Framebuffer<SIZE>,
        x: u16,
        y: u16,
        bitmap_data: &[u8],
        width: usize,
        height: usize,
    ) -> SystemResult<()> {
        for row in 0..height {
            for col in 0..width {
                let byte_index = (row * width + col) / 8;
                let bit_index = 7 - ((row * width + col) % 8);

                if byte_index < bitmap_data.len() {
                    let pixel_value = (bitmap_data[byte_index] >> bit_index) & 1;
                    // 0=黑色（绘制），1=白色（不绘制）
                    if pixel_value == 0 {
                        let px = x as usize + col;
                        let py = y as usize + row;
                        if px < framebuffer.width() as usize && py < framebuffer.height() as usize {
                            framebuffer
                                .draw_pixel(px as u16, py as u16, Color::Black)
                                .ok();
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// 将 WeatherCondition 转换为 WeatherIcon
    fn condition_to_weather_icon(&self, condition: WeatherCondition, is_day: bool) -> WeatherIcon {
        match (condition, is_day) {
            // 晴天
            (WeatherCondition::Sunny, true) => WeatherIcon::Icon100,
            (WeatherCondition::Sunny, false) => WeatherIcon::Icon150,
            // 多云
            (WeatherCondition::Cloudy, true) => WeatherIcon::Icon102,
            (WeatherCondition::Cloudy, false) => WeatherIcon::Icon151,
            // 阴天
            (WeatherCondition::Overcast, _) => WeatherIcon::Icon104,
            // 雨天
            (WeatherCondition::LightRain, _) => WeatherIcon::Icon309,
            (WeatherCondition::ModerateRain, _) => WeatherIcon::Icon306,
            (WeatherCondition::HeavyRain, _) => WeatherIcon::Icon306,
            // 雷暴
            (WeatherCondition::Thunderstorm, _) => WeatherIcon::Icon302,
            // 雪
            (WeatherCondition::Snow, _) => WeatherIcon::Icon400,
            // 雾
            (WeatherCondition::Fog, _) => WeatherIcon::Icon501,
            // 霾
            (WeatherCondition::Haze, _) => WeatherIcon::Icon104,
        }
    }
}
