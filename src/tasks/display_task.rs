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
use crate::kernel::render::layout::engine::DEFAULT_ENGINE;
use crate::tasks::{DISPLAY_EVENTS, DisplayEvent};

static DISPLAY_BUFFER: StaticCell<GlobalMutex<Display7in5>> = StaticCell::new();

/// 显示任务主函数
#[embassy_executor::task]
pub async fn display_task(
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

/// 渲染布局到显示屏
async fn render_layout(
    display_driver: &'static GlobalMutex<DefaultDisplayDriver>,
    display_buffer: &'static GlobalMutex<Display7in5>,
    data_source_registry: &'static GlobalMutex<DataSourceRegistry>,
) {
    log::info!("Rendering layout");

    let mut buffer_guard = display_buffer.lock().await;
    let data_source_guard = data_source_registry.lock().await;

    // 清除显示缓冲区
    buffer_guard.clear(QuadColor::White).unwrap();

    // 使用默认渲染引擎渲染布局到缓冲区
    if let Ok(needs_redraw) = DEFAULT_ENGINE.render_layout(&mut *buffer_guard, &data_source_guard) {
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
    } else {
        log::error!("Failed to render layout");
    }
}
