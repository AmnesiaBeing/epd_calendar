//! 布局管理模块
//! 负责绘制界面布局元素（分隔线、背景等）

extern crate alloc;

use alloc::format;

use super::framebuffer::{Color, Framebuffer};
use super::text::TextRenderer;
use lxx_calendar_common::SystemResult;

/// 布局渲染器
pub struct LayoutRenderer;

impl LayoutRenderer {
    pub fn new() -> Self {
        Self
    }

    /// 绘制分隔线
    /// 参数：
    /// - framebuffer: 渲染缓冲区
    /// - y: 分隔线 Y 坐标
    /// - length: 分隔线长度
    pub fn draw_divider<const SIZE: usize>(
        &self,
        framebuffer: &mut Framebuffer<SIZE>,
        y: u16,
        length: u16,
    ) -> SystemResult<()> {
        framebuffer.draw_horizontal_line(0, y, length, Color::White)?;
        Ok(())
    }

    /// 绘制标题背景区域
    pub fn draw_title_area<const SIZE: usize>(
        &self,
        framebuffer: &mut Framebuffer<SIZE>,
        height: u16,
    ) -> SystemResult<()> {
        // 绘制顶部分隔线
        self.draw_divider(framebuffer, height, framebuffer.width())?;
        Ok(())
    }

    /// 绘制节日标记
    pub fn draw_holiday_marker<const SIZE: usize>(
        &self,
        framebuffer: &mut Framebuffer<SIZE>,
        x: u16,
        y: u16,
        text: &str,
    ) -> SystemResult<()> {
        use heapless::String;

        let mut marker_str = String::<20>::new();
        let _ = marker_str.push_str("[");
        let _ = marker_str.push_str(text);
        let _ = marker_str.push_str("]");

        // 标记区域颜色略浅（白色）
        framebuffer.draw_rectangle(x, y, 60, 16, Color::White)?;

        // 绘制标记文字
        let mut renderer = TextRenderer::new();
        renderer.render(framebuffer, x + 4, y + 2, &marker_str)?;

        Ok(())
    }

    /// 绘制农历信息区域背景
    pub fn draw_lunar_area<const SIZE: usize>(
        &self,
        framebuffer: &mut Framebuffer<SIZE>,
        y: u16,
        height: u16,
    ) -> SystemResult<()> {
        // 绘制背景（略暗）
        framebuffer.draw_rectangle(10, y, 320, height, Color::White)?;
        Ok(())
    }

    /// 绘制宜忌信息
    pub fn draw_yi_ji<const SIZE: usize>(
        &self,
        framebuffer: &mut Framebuffer<SIZE>,
        x: u16,
        y: u16,
        yi: &str,
        ji: &str,
    ) -> SystemResult<()> {
        let mut text = format!("宜：{}", yi);
        if !ji.is_empty() {
            text.push_str(" 忌：");
            text.push_str(ji);
        }

        let mut renderer = TextRenderer::new();
        renderer.render(framebuffer, x, y, &text)?;

        Ok(())
    }

    /// 绘制天气区域背景
    pub fn draw_weather_area<const SIZE: usize>(
        &self,
        framebuffer: &mut Framebuffer<SIZE>,
        y: u16,
        height: u16,
    ) -> SystemResult<()> {
        // 绘制分隔线
        self.draw_divider(framebuffer, y, framebuffer.width())?;

        // 绘制标题（略暗）
        framebuffer.draw_horizontal_line(10, y + 2, 200, Color::White)?;
        framebuffer.draw_horizontal_line(10, y + 4, 200, Color::White)?;

        Ok(())
    }

    /// 绘制格言区域背景
    pub fn draw_quote_area<const SIZE: usize>(
        &self,
        framebuffer: &mut Framebuffer<SIZE>,
        y: u16,
        height: u16,
    ) -> SystemResult<()> {
        // 绘制分隔线
        self.draw_divider(framebuffer, y, framebuffer.width())?;

        // 绘制标题
        framebuffer.draw_horizontal_line(10, y + 2, 100, Color::White)?;

        Ok(())
    }

    /// 绘制状态栏
    pub fn draw_status_bar<const SIZE: usize>(
        &self,
        framebuffer: &mut Framebuffer<SIZE>,
    ) -> SystemResult<()> {
        use heapless::String;

        // 绘制电池图标
        let mut battery_str = String::<6>::new();
        let _ = battery_str.push_str("B%");
        let mut renderer = TextRenderer::new();
        renderer.render(framebuffer, 750, 10, &battery_str)?;

        // 绘制网络图标
        let mut network_str = String::<8>::new();
        let _ = network_str.push_str("WiFi");
        renderer.render(framebuffer, 750, 25, &network_str)?;

        Ok(())
    }

    /// 绘制 QR 码区域
    pub fn draw_qrcode_area<const SIZE: usize>(
        &self,
        framebuffer: &mut Framebuffer<SIZE>,
    ) -> SystemResult<()> {
        // 绘制大字时间背景
        self.draw_title_area(framebuffer, 160)?;

        // 绘制 QR 码占位
        framebuffer.draw_rectangle(280, 160, 200, 200, Color::White)?;

        Ok(())
    }

    /// 绘制错误信息
    pub fn draw_error_message<const SIZE: usize>(
        &self,
        framebuffer: &mut Framebuffer<SIZE>,
        x: u16,
        y: u16,
        message: &str,
    ) -> SystemResult<()> {
        framebuffer.draw_rectangle(x, y, 400, 60, Color::White)?;
        framebuffer.draw_horizontal_line(x, y + 58, 400, Color::White)?;

        let mut renderer = TextRenderer::new();
        renderer.render(framebuffer, x + 20, y + 20, message)?;

        Ok(())
    }

    /// 绘制电池图标
    pub fn draw_battery_icon<const SIZE: usize>(
        &self,
        framebuffer: &mut Framebuffer<SIZE>,
        x: u16,
        y: u16,
        _voltage: u16,
        _charging: bool,
        _low_battery: bool,
    ) -> SystemResult<()> {
        // 绘制电池边框
        framebuffer.draw_rectangle(x, y, 40, 20, Color::Black)?;
        // 绘制电池正极
        framebuffer.draw_rectangle(x + 40, y + 6, 4, 8, Color::Black)?;
        Ok(())
    }
}
