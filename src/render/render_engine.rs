// src/render/render_engine.rs
use embedded_graphics::Drawable;
use epd_waveshare::epd7in5_yrd0750ryf665f60::Display7in5;

use crate::{
    common::{
        SystemState,
        error::{AppError, Result},
    },
    driver::display::{DefaultDisplayDriver, DisplayDriver},
    render::components::separator_component::SeparatorComponent,
    tasks::ComponentDataType,
};

/// 刷新策略说明
/// 刷新任务(display_task) ← 组件更新(各task)
///        ↓
/// 嵌入式芯片内存缓冲区 (render_engine)
///        ↓
/// 屏幕内部缓冲区 (通过驱动接口传输 render_engine.render_component())
///        ↓
/// 实际显示区域 (调用render_engine.display()时更新)

/// 渲染引擎核心结构体
pub struct RenderEngine {
    /// epd-waveshare的Display结构体（自带缓冲区）
    display: Display7in5,
    /// 显示驱动（已封装硬件细节）
    driver: DefaultDisplayDriver,
    /// 是否处于睡眠状态标志
    is_sleeping: bool,
}

impl RenderEngine {
    pub fn new(driver: DefaultDisplayDriver) -> Result<Self> {
        log::info!("Initializing RenderEngine...");

        // 初始化Display（使用默认配置）
        let display = Display7in5::default();

        log::info!("RenderEngine initialized successfully");

        Ok(Self {
            display,
            driver,
            is_sleeping: false,
        })
    }

    fn init_driver(&mut self) -> Result<()> {
        self.driver.init().map_err(|e| {
            log::error!("Failed to initialize display driver: {}", e);
            AppError::RenderingFailed
        })?;
        Ok(())
    }

    /// 使显示驱动进入睡眠状态
    pub fn sleep_driver(&mut self) -> Result<()> {
        if !self.is_sleeping {
            log::info!("Putting display driver to sleep");
            self.driver.sleep().map_err(|e| {
                log::error!("Failed to sleep display driver: {}", e);
                AppError::RenderingFailed
            })?;
            self.is_sleeping = true;
            log::info!("Display driver is now sleeping");
        }
        Ok(())
    }

    /// 渲染单个组件到内存缓冲区
    pub fn render_component(&mut self, component_data: &ComponentDataType) -> Result<()> {
        // 根据组件类型选择并绘制对应组件
        match component_data {
            ComponentDataType::TimeType(component) => {
                component.draw(&mut self.display).map_err(|e| {
                    log::error!("Failed to draw Time component: {}", e);
                    AppError::RenderingFailed
                })?;
            }
            ComponentDataType::DateType(component) => {
                component.draw(&mut self.display).map_err(|e| {
                    log::error!("Failed to draw Date component: {}", e);
                    AppError::RenderingFailed
                })?;
            }
            ComponentDataType::WeatherType(component) => {
                component.draw(&mut self.display).map_err(|e| {
                    log::error!("Failed to draw Weather component: {}", e);
                    AppError::RenderingFailed
                })?;
            }
            ComponentDataType::QuoteType(component) => {
                component.draw(&mut self.display).map_err(|e| {
                    log::error!("Failed to draw Quote component: {}", e);
                    AppError::RenderingFailed
                })?;
            }
            ComponentDataType::ChargingStatusType(component) => {
                component.draw(&mut self.display).map_err(|e| {
                    log::error!("Failed to draw ChargingStatus component: {}", e);
                    AppError::RenderingFailed
                })?;
            }
            ComponentDataType::BatteryType(component) => {
                component.draw(&mut self.display).map_err(|e| {
                    log::error!("Failed to draw BatteryLevel component: {}", e);
                    AppError::RenderingFailed
                })?;
            }
            ComponentDataType::NetworkStatusType(component) => {
                component.draw(&mut self.display).map_err(|e| {
                    log::error!("Failed to draw NetworkStatus component: {}", e);
                    AppError::RenderingFailed
                })?;
            }
        }

        log::info!("Successfully partially rendered component buffer");

        Ok(())
    }

    /// 全屏渲染到缓冲区，不实际在屏幕上显示
    pub fn render_full_screen(&mut self, state: &SystemState) -> Result<()> {
        log::info!("Starting full screen rendering");

        SeparatorComponent.draw(&mut self.display).map_err(|e| {
            log::error!("Failed to draw Separator component: {}", e);
            AppError::RenderingFailed
        })?;

        (&state.time).draw(&mut self.display).map_err(|e| {
            log::error!("Failed to draw Time component: {}", e);
            AppError::RenderingFailed
        })?;

        (&state.date).draw(&mut self.display).map_err(|e| {
            log::error!("Failed to draw Date component: {}", e);
            AppError::RenderingFailed
        })?;

        (&state.weather).draw(&mut self.display).map_err(|e| {
            log::error!("Failed to draw Weather component: {}", e);
            AppError::RenderingFailed
        })?;

        (&state.quote).draw(&mut self.display).map_err(|e| {
            log::error!("Failed to draw Quote component: {}", e);
            AppError::RenderingFailed
        })?;

        (&state.lunar).draw(&mut self.display).map_err(|e| {
            log::error!("Failed to draw Lunar component: {}", e);
            AppError::RenderingFailed
        })?;

        (&state.charging_status)
            .draw(&mut self.display)
            .map_err(|e| {
                log::error!("Failed to draw ChargingStatus component: {}", e);
                AppError::RenderingFailed
            })?;

        (&state.network_status)
            .draw(&mut self.display)
            .map_err(|e| {
                log::error!("Failed to draw NetworkStatus component: {}", e);
                AppError::RenderingFailed
            })?;

        (&state.battery_level)
            .draw(&mut self.display)
            .map_err(|e| {
                log::error!("Failed to draw BatteryLevel component: {}", e);
                AppError::RenderingFailed
            })?;

        log::info!("Full screen buffer rendering completed");

        Ok(())
    }

    /// 在屏幕上刷新显示，将内存中的内容显示出来
    pub async fn refresh_display(&mut self) -> Result<()> {
        log::info!("Refreshing display");

        self.init_driver().map_err(|_| {
            log::error!("Failed to initialize display driver");
            AppError::RenderingFailed
        })?;

        self.driver
            .update_frame(self.display.buffer())
            .map_err(|e| {
                log::error!("Failed to update frame: {}", e);
                AppError::RenderingFailed
            })?;

        self.driver.display_frame().map_err(|e| {
            log::error!("Failed to refresh display: {}", e);
            AppError::RenderingFailed
        })?;

        log::info!("Display refreshed successfully");
        Ok(())
    }
}
