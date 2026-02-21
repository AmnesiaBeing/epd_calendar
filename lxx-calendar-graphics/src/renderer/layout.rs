//! 布局管理模块
//! 负责绘制界面布局元素（分隔线、背景等）

use super::framebuffer::Color;

/// 布局渲染器
pub struct LayoutRenderer;

impl LayoutRenderer {
    pub fn new() -> Self {
        Self
    }

    /// 绘制分隔线
    /// 参数：
    /// - framebuffer: 渲染缓冲区
    /// - y: 分隔线Y坐标
    /// - length: 分隔线长度
    pub fn draw_divider(&self, framebuffer: &mut crate::renderer::Framebuffer, y: u16, length: u16) -> SystemResult<()> {
        framebuffer.draw_horizontal_line(0, y, length, Color::White)?;
        Ok(())
    }

    /// 绘制标题背景区域
    pub fn draw_title_area(&self, framebuffer: &mut crate::renderer::Framebuffer, height: u16) -> SystemResult<()> {
        // 绘制顶部分隔线
        self.draw_divider(framebuffer, height, framebuffer.width())?;
        Ok(())
    }

    /// 绘制节日标记
    pub fn draw_holiday_marker(&self, framebuffer: &mut crate::renderer::Framebuffer, x: u16, y: u16, text: &str) -> SystemResult<()> {
        use heapless::String;

        let mut marker_str = String::from_capacity(20);
        let _ = marker_str.push_str("[");
        let _ = marker_str.push_str(text);
        let _ = marker_str.push_str("]");

        // 标记区域颜色略浅（白色）
        framebuffer.draw_rectangle(x, y, 60, 16, Color::White)?;

        // 绘制标记文字
        let mut renderer = crate::renderer::text::TextRenderer::new();
        renderer.render(framebuffer, x + 4, y + 2, &marker_str)?;

        Ok(())
    }

    /// 绘制农历信息区域背景
    pub fn draw_lunar_area(&self, framebuffer: &mut crate::renderer::Framebuffer, y: u16, height: u16) -> SystemResult<()> {
        // 绘制背景（略暗）
        framebuffer.draw_rectangle(10, y, 320, height, Color::White)?;
        Ok(())
    }

    /// 绘制宜忌信息
    pub fn draw_yi_ji(&self, framebuffer: &mut crate::renderer::Framebuffer, x: u16, y: u16, yi: &str, ji: &str) -> SystemResult<()> {
        let mut text = format!("宜：{}", yi);
        if !ji.is_empty() {
            text.push_str(" 忌：");
            text.push_str(ji);
        }

        let mut renderer = crate::renderer::text::TextRenderer::new();
        renderer.render(framebuffer, x, y, &text)?;

        Ok(())
    }

    /// 绘制天气区域背景
    pub fn draw_weather_area(&self, framebuffer: &mut crate::renderer::Framebuffer, y: u16, height: u16) -> SystemResult<()> {
        // 绘制分隔线
        self.draw_divider(framebuffer, y, framebuffer.width())?;

        // 绘制标题（略暗）
        framebuffer.draw_horizontal_line(10, y + 2, 200, Color::White)?;
        framebuffer.draw_horizontal_line(10, y + 4, 200, Color::White)?;

        Ok(())
    }

    /// 绘制格言区域背景
    pub fn draw_quote_area(&self, framebuffer: &mut crate::renderer::Framebuffer, y: u16, height: u16) -> SystemResult<()> {
        // 绘制分隔线
        self.draw_divider(framebuffer, y, framebuffer.width())?;

        // 绘制标题
        framebuffer.draw_horizontal_line(10, y + 2, 100, Color::White)?;

        Ok(())
    }

    /// 绘制状态栏
    pub fn draw_status_bar(&self, framebuffer: &mut crate::renderer::Framebuffer) -> SystemResult<()> {
        use heapless::String;

        // 绘制电池图标
        let mut battery_str = String::from_capacity(6);
        let _ = battery_str.push_str("B%");
        let mut renderer = crate::renderer::text::TextRenderer::new();
        renderer.render(framebuffer, 750, 10, &battery_str)?;

        // 绘制网络图标
        let mut network_str = String::from_capacity(8);
        let _ = network_str.push_str("WiFi");
        renderer.render(framebuffer, 750, 25, &network_str)?;

        Ok(())
    }

    /// 绘制QR码区域
    pub fn draw_qrcode_area(&self, framebuffer: &mut crate::renderer::Framebuffer) -> SystemResult<()> {
        // 绘制大字时间背景
        self.draw_title_area(framebuffer, 160)?;

        // 绘制QR码占位
        framebuffer.draw_rectangle(280, 160, 200, 200, Color::White)?;

        Ok(())
    }

    /// 绘制错误信息
    pub fn draw_error_message(&self, framebuffer: &mut crate::renderer::Framebuffer, x: u16, y: u16, message: &str) -> SystemResult<()> {
        framebuffer.draw_rectangle(x, y, 400, 60, Color::White)?;
        framebuffer.draw_horizontal_line(x, y + 58, 400, Color::White)?;

        let mut renderer = crate::renderer::text::TextRenderer::new();
        renderer.render(framebuffer, x + 20, y + 20, message)?;

        Ok(())
    }
}
