//! 电池状态渲染器 - 在屏幕指定位置渲染电池电量和充电状态

use embedded_graphics::prelude::*;
use epd_waveshare::color::QuadColor;

use crate::drv::{
    generated_battery_icons::{BATTERY_ICON_SIZE, BatteryIcon, get_battery_icon_data},
    image_renderer::draw_binary_image,
};

// 位置定义
const MARGIN_Y: i32 = 10;
const BATTERY_X: i32 = 800 - BATTERY_ICON_SIZE as i32 - 10;
const CHARGING_X: i32 = 800 - BATTERY_ICON_SIZE as i32 * 2 - 15;

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
