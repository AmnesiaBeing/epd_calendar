//! 网络状态渲染器 - 在屏幕指定位置渲染网络状态图标

use embedded_graphics::prelude::*;
use epd_waveshare::color::QuadColor;

use crate::drv::{
    image_renderer::draw_binary_image,
    generated_network_icons::{NETWORK_ICON_SIZE, NetworkIcon, get_network_icon_data},
};

// 位置定义
const MARGIN_Y: i32 = 10;
const NETWORK_X: i32 = 10;

pub struct NetworkStatus {
    pub is_connected: bool, // 是否已连接网络
}

// 便捷函数：在默认位置渲染网络状态
pub fn render_network_status<D>(display: &mut D, status: &NetworkStatus) -> Result<(), D::Error>
where
    D: DrawTarget<Color = QuadColor>,
{
    // 获取网络图标
    let network_icon = if status.is_connected {
        NetworkIcon::Connected
    } else {
        NetworkIcon::Disconnected
    };

    let _ = draw_binary_image(
        display,
        get_network_icon_data(network_icon),
        Size::new(NETWORK_ICON_SIZE, NETWORK_ICON_SIZE),
        Point::new(NETWORK_X, MARGIN_Y),
    );

    Ok(())
}
