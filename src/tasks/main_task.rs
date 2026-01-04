// src/tasks/display_task.rs

//! 显示任务模块 - 处理屏幕显示和刷新逻辑
//!
//! 该模块负责管理屏幕显示，包括组件渲染、屏幕刷新和防抖控制。

use embedded_graphics::draw_target::DrawTarget;
use epd_waveshare::color::QuadColor;
use epd_waveshare::epd7in5_yrd0750ryf665f60::Display7in5;
use static_cell::StaticCell;

use crate::common::GlobalMutex;
use crate::kernel::data::DataSourceRegistry;
use crate::kernel::driver::display::{DefaultDisplayDriver, DisplayDriver};
use crate::kernel::render::DEFAULT_ENGINE;
use crate::tasks::{DISPLAY_EVENTS, DisplayEvent};

static DISPLAY_BUFFER: StaticCell<GlobalMutex<Display7in5>> = StaticCell::new();

/// 显示任务主函数
#[embassy_executor::task]
pub async fn main_task(
    display_driver: &'static GlobalMutex<DefaultDisplayDriver>,
    data_source_registry: &'static GlobalMutex<DataSourceRegistry>,
) {
    log::info!("🖥️ Display task started");

    let display_buffer = DISPLAY_BUFFER.init(GlobalMutex::new(Display7in5::default()));

    let receiver = DISPLAY_EVENTS.receiver();

    // 初始全屏渲染并刷新
    log::info!("Performing initial display setup");

    // 首次渲染布局
    render_layout(display_driver, display_buffer, data_source_registry).await;

    // 主事件循环
    loop {
        match receiver.receive().await {
            DisplayEvent::FullRefresh => {
                log::info!("DataSource updated, refreshing layout");
                render_layout(display_driver, display_buffer, data_source_registry).await;
            }
        }
    }
}

/// 渲染WiFi配对二维码
// async fn render_wifi_pairing_qr(
//     display_driver: &'static GlobalMutex<DefaultDisplayDriver>,
//     display_buffer: &'static GlobalMutex<Display7in5>,
// ) {
//     log::info!("Rendering WiFi pairing QR code");

//     let mut buffer_guard = display_buffer.lock().await;

//     // 清除显示缓冲区
//     if let Err(e) = buffer_guard.clear(QuadColor::White) {
//         log::error!("Failed to clear display buffer: {:?}", e);
//         return;
//     }

//     // 生成WiFi配对二维码
//     let qr_code = match qrcode::QrCode::new("WIFI:T:WPA;S:EPD_Calendar;P:12345678;;") {
//         Ok(qr) => qr,
//         Err(e) => {
//             log::error!("Failed to generate QR code: {:?}", e);
//             return;
//         }
//     };

//     let qr_image = qr_code
//         .render::<QuadColor>()
//         .dark_color(QuadColor::Black)
//         .light_color(QuadColor::White)
//         .build();

//     // 将二维码绘制到缓冲区
//     let qr_size = qr_image.len();
//     let offset_x = (buffer_guard.width() - qr_size as u32) / 2;
//     let offset_y = (buffer_guard.height() - qr_size as u32) / 2;

//     for (y, row) in qr_image.iter().enumerate() {
//         for (x, &color) in row.iter().enumerate() {
//             buffer_guard.set_pixel(Pixel(
//                 Point::new(offset_x + x as u32, offset_y + y as u32),
//                 color,
//             ));
//         }
//     }

//     // 更新显示
//     let mut display_guard = display_driver.lock().await;
//     if let Err(e) = display_guard.update_frame(buffer_guard.buffer()) {
//         log::error!("Failed to update frame: {:?}", e);
//         return;
//     }

//     if let Err(e) = display_guard.display_frame() {
//         log::error!("Failed to display frame: {:?}", e);
//     }
// }

/// 渲染布局到显示屏
async fn render_layout(
    display_driver: &'static GlobalMutex<DefaultDisplayDriver>,
    display_buffer: &'static GlobalMutex<Display7in5>,
    data_source_registry: &'static GlobalMutex<DataSourceRegistry>,
) {
    log::info!("Rendering layout");

    // 检查WiFi配对状态
    // let config = crate::kernel::data::sources::config::SystemConfig::get_instance().await;
    // let is_wifi_paired = config.get("wifi_ssid").is_some();

    // if !is_wifi_paired {
    //     // 显示WiFi配对二维码
    //     render_wifi_pairing_qr(display_driver, display_buffer).await;
    //     return;
    // }
    log::info!("Rendering layout");

    // 显示时钟页面
    let mut buffer_guard = display_buffer.lock().await;
    let data_source_guard = data_source_registry.lock().await;
    let cache_guard = data_source_guard.get_cache_read_guard().await;

    // 清除显示缓冲区
    buffer_guard.clear(QuadColor::White).unwrap();

    // 使用默认渲染引擎渲染布局到缓冲区
    match DEFAULT_ENGINE.render_layout(&mut *buffer_guard, &data_source_guard, &cache_guard) {
        Ok(needs_redraw) => {
            if needs_redraw {
                log::info!("Layout rendered successfully, updating display");

                // 将缓冲区内容更新到显示驱动并刷新屏幕
                let mut display_guard = display_driver.lock().await;

                // 将缓冲区传递给显示驱动的update_frame方法
                if let Err(e) = display_guard.update_frame(buffer_guard.buffer()) {
                    log::error!("Failed to update frame: {:?}", e);
                    return;
                }

                // 调用display_frame在屏幕上实际渲染
                if let Err(e) = display_guard.display_frame() {
                    log::error!("Failed to display frame: {:?}", e);
                }
            } else {
                log::info!("No redraw needed");
            }
        }
        Err(err) => {
            log::error!("Failed to render layout: {:?}", err);
        }
    }
}
