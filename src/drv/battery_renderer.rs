//! 电池状态渲染器 - 在屏幕指定位置渲染电池电量和充电状态

use embedded_graphics::prelude::*;
use epd_waveshare::color::QuadColor;

use crate::drv::battery_icons::{BATTERY_ICON_SIZE, BatteryIcon, get_battery_icon_data};

// 颜色定义
const BACKGROUND_COLOR: QuadColor = QuadColor::White;
const FOREGROUND_COLOR: QuadColor = QuadColor::Black;

// 位置定义
const MARGIN_Y: i32 = 10;
const BATTERY_X: i32 = 800 - BATTERY_ICON_SIZE as i32 - 10;
const CHARGING_X: i32 = 800 - BATTERY_ICON_SIZE as i32 * 2 - 15;

pub fn draw_binary_image<D>(
    display: &mut D,
    icon_data: &[u8],
    size: Size,
    position: Point,
) -> Result<(), D::Error>
where
    D: DrawTarget<Color = QuadColor>,
{
    let width = size.width as usize;
    let height = size.height as usize;

    // 计算每行字节数
    let bytes_per_row = (width + 7) / 8;

    // 绘制每个像素
    let pixels = (0..height).flat_map(move |y| {
        (0..width).map(move |x| {
            let byte_index = (y * bytes_per_row + x / 8) as usize;
            let bit_index = 7 - (x % 8);

            let color = if byte_index < icon_data.len() {
                let byte = icon_data[byte_index];
                let bit = (byte >> bit_index) & 1;
                if bit == 1 {
                    FOREGROUND_COLOR
                } else {
                    BACKGROUND_COLOR
                }
            } else {
                QuadColor::White
            };

            Pixel(Point::new(x as i32, y as i32) + position, color)
        })
    });

    display.draw_iter(pixels)
}

pub struct BatteryStatus {
    pub level: u8,         // 电池电量百分比 (0-100)
    pub is_charging: bool, // 是否正在充电
}

// 便捷函数：在默认位置渲染电池状态
pub fn render_battery_status<D>(display: &mut D, status: &BatteryStatus) -> Result<(), D::Error>
where
    D: DrawTarget<Color = QuadColor>,
{
    // 获取对应的电池图标
    let battery_icon = match status.level {
        0 => BatteryIcon::Level0,
        1..=25 => BatteryIcon::Level1,
        26..=50 => BatteryIcon::Level2,
        51..=75 => BatteryIcon::Level3,
        _ => BatteryIcon::Level4,
    };

    // 获取充电图标
    let charging_icon = if status.is_charging {
        Some(BatteryIcon::Charging)
    } else {
        None
    };

    let _ = draw_binary_image(
        display,
        get_battery_icon_data(battery_icon),
        Size::new(BATTERY_ICON_SIZE, BATTERY_ICON_SIZE),
        Point::new(BATTERY_X, MARGIN_Y),
    );

    if status.is_charging {
        let _ = draw_binary_image(
            display,
            get_battery_icon_data(charging_icon.unwrap()),
            Size::new(BATTERY_ICON_SIZE, BATTERY_ICON_SIZE),
            Point::new(CHARGING_X, MARGIN_Y),
        );
    }

    Ok(())
}
