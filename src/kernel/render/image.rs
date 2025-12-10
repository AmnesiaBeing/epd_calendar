//! 图像/图标渲染器
//! 负责将图像和图标绘制到屏幕上

use crate::assets::generated_icons::{IconId, BatteryIcon, NetworkIcon, TimeDigitIcon, WeatherIcon};
use crate::kernel::render::layout::nodes::Importance;
use embedded_graphics::{draw_target::DrawTarget, prelude::*};
use epd_waveshare::color::QuadColor;

/// 图像渲染错误
#[derive(Debug, PartialEq, Eq)]
pub enum ImageRenderError {
    /// 图标未找到
    IconNotFound,
    /// 渲染失败
    RenderFailed,
    /// 超出边界
    OutOfBounds,
}

/// 图像渲染器
pub struct ImageRenderer {
    // 可以添加图标缓存或其他状态
}

impl ImageRenderer {
    /// 创建新的图像渲染器
    pub const fn new() -> Self {
        Self {}
    }

    /// 渲染图标
    pub fn render<D: DrawTarget<Color = QuadColor>>(
        &self,
        draw_target: &mut D,
        rect: [u16; 4],
        icon_id: &str,
        importance: Option<Importance>,
    ) -> Result<(), ImageRenderError> {
        // 获取图标数据
        let icon_id = self
            .get_icon_data(icon_id)
            .ok_or(ImageRenderError::IconNotFound)?;

        // 计算图标位置（居中）
        let [x, y, width, height] = rect;
        let icon_size = icon_id.size();
        let icon_width = icon_size.width as u16;
        let icon_height = icon_size.height as u16;

        let x_pos = x + (width - icon_width) / 2;
        let y_pos = y + (height - icon_height) / 2;

        // 绘制图标
        self.draw_icon(draw_target, x_pos, y_pos, icon_id)
    }

    /// 获取图标数据
    fn get_icon_data(&self, icon_id: &str) -> Option<IconId> {
        // 将字符串ID转换为IconId
        // 这里实现简单的映射，根据实际情况可能需要更复杂的解析
        match icon_id {
            "battery-0" => Some(IconId::BATTERY(BatteryIcon::Battery0)),
            "battery-1" => Some(IconId::BATTERY(BatteryIcon::Battery1)),
            "battery-2" => Some(IconId::BATTERY(BatteryIcon::Battery2)),
            "battery-3" => Some(IconId::BATTERY(BatteryIcon::Battery3)),
            "battery-4" => Some(IconId::BATTERY(BatteryIcon::Battery4)),
            "bolt" => Some(IconId::BATTERY(BatteryIcon::Bolt)),
            "connected" => Some(IconId::NETWORK(NetworkIcon::Connected)),
            "disconnected" => Some(IconId::NETWORK(NetworkIcon::Disconnected)),
            "digit_0" => Some(IconId::TIME_DIGIT(TimeDigitIcon::Digit0)),
            "digit_1" => Some(IconId::TIME_DIGIT(TimeDigitIcon::Digit1)),
            "digit_2" => Some(IconId::TIME_DIGIT(TimeDigitIcon::Digit2)),
            "digit_3" => Some(IconId::TIME_DIGIT(TimeDigitIcon::Digit3)),
            "digit_4" => Some(IconId::TIME_DIGIT(TimeDigitIcon::Digit4)),
            "digit_5" => Some(IconId::TIME_DIGIT(TimeDigitIcon::Digit5)),
            "digit_6" => Some(IconId::TIME_DIGIT(TimeDigitIcon::Digit6)),
            "digit_7" => Some(IconId::TIME_DIGIT(TimeDigitIcon::Digit7)),
            "digit_8" => Some(IconId::TIME_DIGIT(TimeDigitIcon::Digit8)),
            "digit_9" => Some(IconId::TIME_DIGIT(TimeDigitIcon::Digit9)),
            "digit_colon" => Some(IconId::TIME_DIGIT(TimeDigitIcon::DigitColon)),
            "digit_sep" => Some(IconId::TIME_DIGIT(TimeDigitIcon::DigitSep)),
            // 天气图标支持，如 "100", "101", etc.
            _ if icon_id.starts_with("icon_") => {
                let weather_code = &icon_id[5..];
                WeatherIcon::from_api_str(weather_code).ok().map(IconId::Weather)
            },
            _ => WeatherIcon::from_api_str(icon_id).ok().map(IconId::Weather),
        }
    }

    /// 绘制图标
    fn draw_icon<D: DrawTarget<Color = QuadColor>>(
        &self,
        draw_target: &mut D,
        x: u16,
        y: u16,
        icon_id: IconId,
    ) -> Result<(), ImageRenderError> {
        let icon_size = icon_id.size();
        let bitmap_data = icon_id.data();
        let width = icon_size.width;
        let _height = icon_size.height;

        // 按字节绘制位图数据
        for (byte_idx, byte) in bitmap_data.iter().enumerate() {
            let y_offset = byte_idx / (width as usize / 8);
            let x_offset = (byte_idx % (width as usize / 8)) * 8;

            // 处理每个字节的8个像素
            for bit in 0..8 {
                if (byte >> (7 - bit)) & 1 != 0 {
                    let pixel_x = x + (x_offset + bit) as u16;
                    let pixel_y = y + y_offset as u16;

                    // 确保像素在绘制目标范围内
                    let point = Point::new(pixel_x as i32, pixel_y as i32);
                    let pixel = Pixel(point, QuadColor::Black);
                    let _ = draw_target.draw_iter(core::iter::once(pixel));
                }
            }
        }

        Ok(())
    }
}
