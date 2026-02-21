//! 图标渲染模块
//! 负责渲染各种图标（时间、天气、电池等）

use core::fmt::Write;
use super::framebuffer::Color;

/// 图标渲染器
pub struct IconRenderer;

impl IconRenderer {
    pub fn new() -> Self {
        Self
    }

    /// 渲染天气图标
    /// 参数：
    /// - framebuffer: 渲染缓冲区
    /// - x: X坐标
    /// - y: Y坐标
    /// - weather_icon: 天气图标类型
    pub fn render_weather_icon(
        &self,
        framebuffer: &mut crate::renderer::Framebuffer,
        x: u16,
        y: u16,
        weather_icon: u8, // WeatherIcon 枚举值
    ) -> Result<(), ()> {
        // 简化实现：绘制一个圆形表示天气图标
        // 实际项目中应该使用生成的天气图标位图
        self.draw_rounded_rectangle(framebuffer, x, y, 32, 32, 16, Color::White)?;

        // 根据天气类型绘制简单的图形
        match weather_icon {
            // 晴天
            WeatherIcon::Icon100 | WeatherIcon::Icon1001 => {
                self.draw_sun(framebuffer, x + 16, y + 16)?;
            }
            // 多云
            WeatherIcon::Icon1002 | WeatherIcon::Icon1003 => {
                self.draw_cloud(framebuffer, x + 16, y + 20)?;
            }
            // 阴天
            WeatherIcon::Icon1004 | WeatherIcon::Icon1005 => {
                self.draw_cloud(framebuffer, x + 16, y + 18)?;
                self.draw_cloud(framebuffer, x + 12, y + 22)?;
            }
            // 小雨
            WeatherIcon::Icon101 => {
                self.draw_cloud(framebuffer, x + 16, y + 18)?;
                self.draw_rain(framebuffer, x + 8, y + 28)?;
            }
            // 中雨
            WeatherIcon::Icon102 => {
                self.draw_cloud(framebuffer, x + 16, y + 18)?;
                self.draw_rain(framebuffer, x + 8, y + 28)?;
                self.draw_rain(framebuffer, x + 24, y + 28)?;
            }
            // 大雨
            WeatherIcon::Icon103 => {
                self.draw_cloud(framebuffer, x + 16, y + 18)?;
                self.draw_rain(framebuffer, x + 6, y + 28)?;
                self.draw_rain(framebuffer, x + 18, y + 28)?;
                self.draw_rain(framebuffer, x + 30, y + 28)?;
            }
            // 雷暴
            WeatherIcon::Icon108 | WeatherIcon::Icon110 => {
                self.draw_cloud(framebuffer, x + 16, y + 18)?;
                self.draw_rain(framebuffer, x + 8, y + 28)?;
                self.draw_lightning(framebuffer, x + 20, y + 24)?;
            }
            // 雪
            WeatherIcon::Icon111 => {
                self.draw_cloud(framebuffer, x + 16, y + 18)?;
                self.draw_snow(framebuffer, x + 12, y + 32)?;
                self.draw_snow(framebuffer, x + 20, y + 30)?;
            }
            // 雾
            WeatherIcon::Icon115 => {
                self.draw_fog(framebuffer, x + 8, y + 16)?;
                self.draw_fog(framebuffer, x + 24, y + 16)?;
            }
            // 霾
            WeatherIcon::Icon302 | WeatherIcon::Icon303 => {
                self.draw_cloud(framebuffer, x + 16, y + 18)?;
            }
            _ => {}
        }

        Ok(())
    }

    /// 渲染太阳
    fn draw_sun(&self, framebuffer: &mut crate::renderer::Framebuffer, x: u16, y: u16) -> SystemResult<()> {
        // 太阳本体
        for i in 0..16 {
            for j in 0..16 {
                let px = x as i32 + i as i32 - 8;
                let py = y as i32 + j as i32 - 8;
                let dist = ((px - 16) * (px - 16) + (py - 16) * (py - 16)) as f32;
                if dist < 256.0 {
                    framebuffer.draw_pixel(px as u16, py as u16, Color::White)?;
                }
            }
        }
        Ok(())
    }

    /// 渲染云
    fn draw_cloud(&self, framebuffer: &mut crate::renderer::Framebuffer, x: u16, y: u16) -> SystemResult<()> {
        // 绘制简单的云形
        for i in 0..12 {
            for j in 0..8 {
                let px = x as i32 + i as i32;
                let py = y as i32 + j as i32;
                // 简单的圆形云
                if (px - 6) * (px - 6) + (py - 4) * (py - 4) < 36 {
                    framebuffer.draw_pixel(px, py, Color::White)?;
                }
            }
        }
        Ok(())
    }

    /// 渲染雨
    fn draw_rain(&self, framebuffer: &mut crate::renderer::Framebuffer, x: u16, y: u16) -> SystemResult<()> {
        for i in 0..4 {
            let px = x + (i as u16) * 6;
            let py = y + (i as u16) * 6 + 4;
            framebuffer.draw_pixel(px, py, Color::White)?;
            framebuffer.draw_pixel(px + 4, py + 2, Color::White)?;
        }
        Ok(())
    }

    /// 渲染闪电
    fn draw_lightning(&self, framebuffer: &mut crate::renderer::Framebuffer, x: u16, y: u16) -> SystemResult<()> {
        // 三角形闪电
        let points = [(x, y), (x + 4, y + 8), (x + 2, y + 6), (x + 12, y + 12), (x + 8, y + 10)];
        for &(px, py) in points.iter() {
            framebuffer.draw_pixel(px, py, Color::White)?;
        }
        Ok(())
    }

    /// 渲染雪
    fn draw_snow(&self, framebuffer: &mut crate::renderer::Framebuffer, x: u16, y: u16) -> SystemResult<()> {
        for i in 0..3 {
            let px = x + (i as u16) * 8 + 4;
            let py = y + (i as u16) * 5;
            // 小点表示雪花
            framebuffer.draw_pixel(px, py, Color::White)?;
            framebuffer.draw_pixel(px + 3, py + 3, Color::White)?;
        }
        Ok(())
    }

    /// 渲染雾
    fn draw_fog(&self, framebuffer: &mut crate::renderer::Framebuffer, x: u16, y: u16) -> SystemResult<()> {
        for i in 0..24 {
            for j in 0..6 {
                let px = x as i32 + i as i32;
                let py = y as i32 + j as i32;
                if px >= 0 && px < framebuffer.width() as i32 && py >= 0 && py < framebuffer.height() as i32 {
                    framebuffer.draw_pixel(px as u16, py as u16, Color::White)?;
                }
            }
        }
        Ok(())
    }

    /// 绘制圆角矩形
    fn draw_rounded_rectangle(
        &self,
        framebuffer: &mut crate::renderer::Framebuffer,
        x: u16,
        y: u16,
        width: u16,
        height: u16,
        radius: u16,
        color: Color,
    ) -> SystemResult<()> {
        // 绘制矩形边框
        framebuffer.draw_rectangle(x, y, width, height, color)?;
        Ok(())
    }
}
