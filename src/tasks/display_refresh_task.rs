// src/tasks/display_refresh_task.rs
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Timer};
use log::{debug, info};

use crate::app_core::display_manager::{DisplayManager, RefreshPlan};
use crate::common::types::DisplayData;
use crate::render::RenderEngine;

#[embassy_executor::task]
pub async fn display_refresh_task(
    display_manager: &'static Mutex<ThreadModeRawMutex, DisplayManager>,
    display_data: &'static Mutex<ThreadModeRawMutex, DisplayData<'static>>,
    render_engine: &'static Mutex<ThreadModeRawMutex, RenderEngine>,
) {
    debug!("Display refresh task started");

    let mut refresh_interval = Duration::from_secs(2); // 默认2秒检查一次
    let mut consecutive_errors = 0;
    const MAX_CONSECUTIVE_ERRORS: u8 = 3;

    loop {
        Timer::after(refresh_interval).await;

        // 获取刷新计划
        let refresh_plan = {
            let mut dm = display_manager.lock().await;
            dm.get_refresh_plan()
        };

        match refresh_plan {
            RefreshPlan::NoUpdate => {
                // 没有需要刷新的内容，可以稍微延长检查间隔
                if refresh_interval < Duration::from_secs(10) {
                    refresh_interval = Duration::from_secs(5);
                }
                continue;
            }
            RefreshPlan::Global => {
                info!("Performing global display refresh");

                // 获取最新数据
                let data = {
                    let data_guard = display_data.lock().await;
                    data_guard.clone()
                };

                // 执行全局渲染
                let result = {
                    let mut engine = render_engine.lock().await;
                    engine.render_full_display(&data).await
                };

                match result {
                    Ok(()) => {
                        consecutive_errors = 0;
                        refresh_interval = Duration::from_secs(2); // 成功后退回正常间隔
                        info!("Global refresh completed successfully");
                    }
                    Err(e) => {
                        consecutive_errors += 1;
                        log::error!("Global refresh failed: {}", e);

                        if consecutive_errors >= MAX_CONSECUTIVE_ERRORS {
                            log::error!(
                                "Too many consecutive display errors, increasing check interval"
                            );
                            refresh_interval = Duration::from_secs(30); // 增加间隔避免频繁失败
                        }
                    }
                }
            }
            RefreshPlan::Partial(area) => {
                debug!("Performing partial display refresh: {:?}", area);

                // 获取最新数据
                let data = {
                    let data_guard = display_data.lock().await;
                    data_guard.clone()
                };

                // 执行局部渲染
                let result = {
                    let mut engine = render_engine.lock().await;
                    engine.render_partial_display(&data, area).await
                };

                match result {
                    Ok(()) => {
                        consecutive_errors = 0;
                        refresh_interval = Duration::from_secs(2); // 成功后退回正常间隔
                    }
                    Err(e) => {
                        consecutive_errors += 1;
                        log::error!("Partial refresh failed: {}", e);

                        // 局部刷新失败时，标记需要全局刷新
                        {
                            let mut dm = display_manager.lock().await;
                            dm.force_global_refresh();
                        }

                        if consecutive_errors >= MAX_CONSECUTIVE_ERRORS {
                            log::error!(
                                "Too many consecutive display errors, increasing check interval"
                            );
                            refresh_interval = Duration::from_secs(30);
                        }
                    }
                }
            }
        }

        // 重置强制刷新标志
        {
            let mut data = display_data.lock().await;
            data.force_refresh = false;
        }
    }
}
