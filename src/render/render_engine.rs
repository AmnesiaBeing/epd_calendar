// src/render/render_engine.rs

use embedded_graphics::{
    Drawable, draw_target::DrawTarget, geometry::Dimensions, prelude::DrawTargetExt,
};
use epd_waveshare::{color::QuadColor, epd7in5_yrd0750ryf665f60::Display7in5};

// 项目内部依赖
use crate::{
    common::{
        BatteryLevel, ChargingStatus, DateData, Hitokoto, NetworkStatus, SystemState, TimeData,
        WeatherData,
        error::{AppError, Result},
    },
    driver::display::{DefaultDisplayDriver, DisplayDriver},
    tasks::ComponentData,
};

// 定义组件类型枚举
pub enum ComponentType {
    Time(TimeData),
    Date(DateData),
    Weather(WeatherData),
    Quote(&'static Hitokoto),
    ChargingStatus(ChargingStatus),
    BatteryLevel(BatteryLevel),
    NetworkStatus(NetworkStatus),
    Separator,
}

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
    pub fn new(mut driver: DefaultDisplayDriver) -> Result<Self> {
        log::info!("Initializing RenderEngine...");

        // 初始化Display（使用默认配置）
        let display = Display7in5::default();

        // 初始化驱动硬件
        driver.init().map_err(|e| {
            log::error!("Failed to initialize display driver: {}", e);
            AppError::RenderingFailed
        })?;

        log::info!("RenderEngine initialized successfully");

        Ok(Self {
            display,
            driver,
            is_sleeping: false,
        })
    }

    /// 唤醒显示驱动
    pub fn wake_up_driver(&mut self) -> Result<()> {
        if self.is_sleeping {
            log::info!("Waking up display driver");
            self.driver.wake_up().map_err(|e| {
                log::error!("Failed to wake up display driver: {}", e);
                AppError::RenderingFailed
            })?;
            self.is_sleeping = false;
            log::info!("Display driver is now awake");
        }
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

    /// 渲染单个组件（局部刷新）
    pub fn render_component(&mut self, component_data: &ComponentData) -> Result<()> {
        // 确保驱动已唤醒
        self.wake_up_driver()?;

        let bounds;

        // 根据组件类型选择并绘制对应组件
        match component_data {
            ComponentData::TimeData(component) => {
                bounds = component.bounding_box();
                let mut clipped_target = self.display.cropped(&bounds);
                component.draw(&mut clipped_target).map_err(|e| {
                    log::error!("Failed to draw Time component: {}", e);
                    AppError::RenderingFailed
                })?;
            }
            ComponentData::DateData(component) => {
                bounds = component.bounding_box();
                let mut clipped_target = self.display.cropped(&bounds);
                component.draw(&mut clipped_target).map_err(|e| {
                    log::error!("Failed to draw Date component: {}", e);
                    AppError::RenderingFailed
                })?;
            }
            ComponentData::WeatherData(component) => {
                bounds = component.bounding_box();
                let mut clipped_target = self.display.cropped(&bounds);
                component.draw(&mut clipped_target).map_err(|e| {
                    log::error!("Failed to draw Weather component: {}", e);
                    AppError::RenderingFailed
                })?;
            }
            ComponentData::QuoteData(component) => {
                bounds = component.bounding_box();
                let mut clipped_target = self.display.cropped(&bounds);
                component.draw(&mut clipped_target).map_err(|e| {
                    log::error!("Failed to draw Quote component: {}", e);
                    AppError::RenderingFailed
                })?;
            }
            ComponentData::ChargingStatus(component) => {
                bounds = component.bounding_box();
                let mut clipped_target = self.display.cropped(&bounds);
                component.draw(&mut clipped_target).map_err(|e| {
                    log::error!("Failed to draw ChargingStatus component: {}", e);
                    AppError::RenderingFailed
                })?;
            }
            ComponentData::BatteryData(component) => {
                bounds = component.bounding_box();
                let mut clipped_target = self.display.cropped(&bounds);
                component.draw(&mut clipped_target).map_err(|e| {
                    log::error!("Failed to draw BatteryLevel component: {}", e);
                    AppError::RenderingFailed
                })?;
            }
            ComponentData::NetworkStatus(component) => {
                bounds = component.bounding_box();
                let mut clipped_target = self.display.cropped(&bounds);
                component.draw(&mut clipped_target).map_err(|e| {
                    log::error!("Failed to draw NetworkStatus component: {}", e);
                    AppError::RenderingFailed
                })?;
            }
            _ => {
                log::error!("Unhandled component type.");
                return Err(AppError::RenderingFailed);
            }
        }

        // 更新局部区域到墨水屏
        // TODO: 需计算实际的buffer指针，逐个像素绘制到墨水屏
        let buffer = self.display.buffer();
        self.driver
            .update_partial_frame(
                buffer,
                bounds.top_left.x as u32,
                bounds.top_left.y as u32,
                bounds.size.width as u32,
                bounds.size.height as u32,
            )
            .map_err(|e| {
                log::error!("Failed to update partial frame for component");
                AppError::RenderingFailed
            })?;

        log::info!("Successfully partially rendered component");

        Ok(())
    }

    /// 全屏渲染（用于首次显示或清除残影）
    pub fn render_full_screen(&mut self, state: &SystemState) -> Result<()> {
        // 确保驱动已唤醒
        self.wake_up_driver()?;

        log::info!("Starting full screen rendering");

        // 清空显示缓冲区（全白背景）
        self.display.clear(QuadColor::White).map_err(|e| {
            log::error!("Failed to clear display buffer: {}", e);
            AppError::RenderingFailed
        })?;

        // TODO: 按顺序渲染所有组件到全屏缓冲区
        // 注意：这里假设display_task会提供所有组件的边界信息
        // 实际应用中，可能需要遍历一个组件列表

        log::info!("Full screen rendering completed");

        // 更新整个帧缓冲区到墨水屏
        let buffer = self.display.buffer();
        self.driver.update_frame(buffer).map_err(|e| {
            log::error!("Failed to update full frame: {}", e);
            AppError::RenderingFailed
        })?;

        // 触发显示刷新
        self.driver.display_frame().map_err(|e| {
            log::error!("Failed to display frame: {}", e);
            AppError::RenderingFailed
        })?;

        log::info!("Full screen refresh completed successfully");
        Ok(())
    }

    /// 仅刷新显示（不重新渲染，用于局部刷新后的显示更新）
    pub fn refresh_display(&mut self) -> Result<()> {
        log::info!("Refreshing display");
        self.driver.display_frame().map_err(|e| {
            log::error!("Failed to refresh display: {}", e);
            AppError::RenderingFailed
        })?;
        log::info!("Display refreshed successfully");
        Ok(())
    }
}
