use embedded_graphics::Drawable;
use embedded_graphics::draw_target::DrawTarget;
use embedded_graphics::prelude::Point;
use embedded_graphics::primitives::Rectangle;
use epd_waveshare::color::QuadColor;
use epd_waveshare::epd7in5_yrd0750ryf665f60::Display7in5;

use crate::common::error::{AppError, Result};
use crate::driver::display::{DefaultDisplayDriver, DisplayDriver};

/// 渲染引擎 - 负责管理显示缓冲区、脏区域标记和协调渲染刷新
pub struct RenderEngine {
    /// 显示缓冲区
    display_buffer: Display7in5,
    /// 显示驱动
    display_driver: DefaultDisplayDriver,
    /// 脏区域标记（用于部分刷新）
    dirty_region: Option<Rectangle>,
    /// 屏幕尺寸
    screen_size: (u32, u32),
}

impl RenderEngine {
    /// 创建新的渲染引擎实例
    pub fn new(display_driver: DefaultDisplayDriver) -> Self {
        // 7.5英寸墨水屏的分辨率
        const SCREEN_WIDTH: u32 = 800;
        const SCREEN_HEIGHT: u32 = 480;

        Self {
            display_buffer: Display7in5::default(),
            display_driver,
            dirty_region: None,
            screen_size: (SCREEN_WIDTH, SCREEN_HEIGHT),
        }
    }

    /// 标记脏区域（用于部分刷新）
    pub fn mark_dirty(&mut self, region: Rectangle) {
        // 确保区域在屏幕范围内
        let adjusted_region = self.adjust_region_to_screen(region);

        // 合并已有的脏区域
        match self.dirty_region.take() {
            Some(existing) => {
                // 计算合并后的区域
                let merged = self.merge_regions(existing, adjusted_region);
                self.dirty_region = Some(merged);
            }
            None => {
                self.dirty_region = Some(adjusted_region);
            }
        }

        log::debug!("Marked dirty region: {:?}", self.dirty_region);
    }

    /// 清空脏区域标记
    pub fn clear_dirty(&mut self) {
        self.dirty_region = None;
        log::debug!("Dirty region cleared");
    }

    /// 全屏刷新屏幕
    pub async fn full_refresh(&mut self) -> Result<()> {
        log::info!("Performing full display refresh");

        // 清空脏区域
        self.clear_dirty();

        // 执行全屏刷新
        match self
            .display_driver
            .update_frame(self.display_buffer.buffer())
        {
            Ok(_) => {}
            Err(e) => {
                log::error!("Failed to update frame: {:?}", e);
                return Err(AppError::DisplayFullRefreshFailed);
            }
        }

        match self.display_driver.display_frame() {
            Ok(_) => {
                log::debug!("Full refresh completed successfully");
                Ok(())
            }
            Err(e) => {
                log::error!("Failed to display frame: {:?}", e);
                Err(AppError::DisplayFullRefreshFailed)
            }
        }
    }

    /// 部分刷新屏幕
    pub async fn partial_refresh(&mut self) -> Result<()> {
        if let Some(dirty_region) = self.dirty_region.take() {
            log::debug!("Performing partial refresh for region: {:?}", dirty_region);

            // 确保区域对齐到8像素边界（墨水屏部分刷新的常见要求）
            let aligned_region = self.align_to_eight_pixels(dirty_region);

            // 执行部分刷新
            match self.display_driver.update_partial_frame(
                self.display_buffer.buffer(),
                aligned_region.top_left.x as u32,
                aligned_region.top_left.y as u32,
                aligned_region.size.width,
                aligned_region.size.height,
            ) {
                Ok(_) => {}
                Err(e) => {
                    log::error!("Failed to update partial frame: {:?}", e);
                    // 重新标记脏区域，以便下次尝试
                    self.dirty_region = Some(dirty_region);
                    return Err(AppError::DisplayPartialRefreshFailed);
                }
            }

            match self.display_driver.display_frame() {
                Ok(_) => {
                    log::debug!("Partial refresh completed successfully");
                    Ok(())
                }
                Err(e) => {
                    log::error!("Failed to display frame: {:?}", e);
                    // 重新标记脏区域，以便下次尝试
                    self.dirty_region = Some(dirty_region);
                    Err(AppError::DisplayPartialRefreshFailed)
                }
            }
        } else {
            log::debug!("No dirty region to refresh");
            Ok(())
        }
    }

    /// 渲染所有组件到全屏
    pub fn render_full(&mut self, components: &[impl Drawable<Color = QuadColor>]) -> Result<()> {
        log::info!("Rendering full screen");

        // 清空显示缓冲区
        self.clear_buffer();

        // 遍历所有组件并绘制到缓冲区
        for component in components {
            match component.draw(&mut self.display_buffer) {
                Ok(_) => {}
                Err(e) => {
                    log::error!("Failed to draw component: {:?}", e);
                    return Err(AppError::RenderingFailed);
                }
            }
        }

        // 标记整个屏幕为脏区域
        let full_screen = Rectangle::new(
            Point::new(0, 0),
            Size::new(self.screen_size.0, self.screen_size.1),
        );
        self.mark_dirty(full_screen);

        log::debug!("Full screen rendering completed");
        Ok(())
    }

    /// 渲染单个组件到指定区域
    pub fn render_partial(
        &mut self,
        component: impl Drawable<Color = QuadColor>,
        render_area: Rectangle,
    ) -> Result<()> {
        log::debug!("Rendering component to area: {:?}", render_area);

        // 标记脏区域
        self.mark_dirty(render_area);

        // 绘制组件到缓冲区
        match component.draw(&mut self.display_buffer) {
            Ok(_) => {
                log::debug!("Partial rendering completed");
                Ok(())
            }
            Err(e) => {
                log::error!("Failed to draw component: {:?}", e);
                Err(AppError::RenderingFailed)
            }
        }
    }

    /// 清空显示缓冲区
    pub fn clear_buffer(&mut self) {
        self.display_buffer.clear(QuadColor::White);
        log::debug!("Display buffer cleared");
    }

    /// 获取显示缓冲区引用
    pub fn get_buffer(&mut self) -> &mut Display7in5 {
        &mut self.display_buffer
    }

    /// 确保区域在屏幕范围内
    fn adjust_region_to_screen(&self, region: Rectangle) -> Rectangle {
        let (width, height) = self.screen_size;

        let left = core::cmp::max(0, region.top_left.x) as i32;
        let top = core::cmp::max(0, region.top_left.y) as i32;

        let right = core::cmp::min(
            (width - 1) as i32,
            (region.top_left.x + region.size.width as i32 - 1) as i32,
        );
        let bottom = core::cmp::min(
            (height - 1) as i32,
            (region.top_left.y + region.size.height as i32 - 1) as i32,
        );

        if left > right || top > bottom {
            // 区域完全在屏幕外
            Rectangle::new(Point::new(0, 0), Size::new(0, 0))
        } else {
            Rectangle::new(
                Point::new(left, top),
                Size::new((right - left + 1) as u32, (bottom - top + 1) as u32),
            )
        }
    }

    /// 合并两个区域
    fn merge_regions(&self, region1: Rectangle, region2: Rectangle) -> Rectangle {
        let left = core::cmp::min(region1.top_left.x, region2.top_left.x);
        let top = core::cmp::min(region1.top_left.y, region2.top_left.y);

        let right = core::cmp::max(
            region1.top_left.x + region1.size.width as i32 - 1,
            region2.top_left.x + region2.size.width as i32 - 1,
        );

        let bottom = core::cmp::max(
            region1.top_left.y + region1.size.height as i32 - 1,
            region2.top_left.y + region2.size.height as i32 - 1,
        );

        Rectangle::new(
            Point::new(left, top),
            Size::new((right - left + 1) as u32, (bottom - top + 1) as u32),
        )
    }

    /// 对齐区域到8像素边界（墨水屏部分刷新要求）
    fn align_to_eight_pixels(&self, region: Rectangle) -> Rectangle {
        // 左边界向下对齐到8的倍数
        let aligned_left = (region.top_left.x as i32 / 8) * 8;
        // 上边界向下对齐到8的倍数
        let aligned_top = (region.top_left.y as i32 / 8) * 8;

        // 右边界向上对齐到8的倍数
        let aligned_right = ((region.top_left.x as i32 + region.size.width as i32 + 7) / 8) * 8;
        // 下边界向上对齐到8的倍数
        let aligned_bottom = ((region.top_left.y as i32 + region.size.height as i32 + 7) / 8) * 8;

        Rectangle::new(
            Point::new(aligned_left, aligned_top),
            Size::new(
                (aligned_right - aligned_left) as u32,
                (aligned_bottom - aligned_top) as u32,
            ),
        )
    }
}

// 导入Size类型以避免编译错误
use embedded_graphics::prelude::Size;
