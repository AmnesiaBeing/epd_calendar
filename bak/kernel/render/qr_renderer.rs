use crate::common::error::AppError;
use crate::common::error::Result as AppResult;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{PrimitiveStyle, Rectangle, StyledDrawable};
use epd_waveshare::color::QuadColor;
use qrcode::QrCode;
use qrcode::types::Color;

/// QR 码渲染器
pub struct QrRenderer {
    x: i32,
    y: i32,
    scale: u32,
}

impl QrRenderer {
    /// 创建新的 QR 码渲染器
    pub fn new(x: i32, y: i32, scale: u32) -> Self {
        Self { x, y, scale }
    }

    /// 渲染 QR 码到显示设备
    pub fn render<D: DrawTarget<Color = QuadColor>>(
        &self,
        display: &mut D,
        content: &str,
    ) -> AppResult<()> {
        // 生成 QR 码
        let qr = QrCode::new(content).map_err(|_| {
            log::error!("Failed to generate QR code");
            AppError::ConvertError
        })?;
        let size = qr.version().width() as u32;

        // 获取 QR 码模块数据
        let modules = qr.to_colors();

        // 绘制每个模块
        for (i, color) in modules.iter().enumerate() {
            if *color == Color::Dark {
                let x_pos = self.x + (i % size as usize) as i32 * self.scale as i32;
                let y_pos = self.y + (i / size as usize) as i32 * self.scale as i32;

                Rectangle::new(Point::new(x_pos, y_pos), Size::new(self.scale, self.scale))
                    .draw_styled(&PrimitiveStyle::with_fill(QuadColor::Black), display)
                    .map_err(|_| {
                        log::error!("Failed to draw QR code module");
                        AppError::ConvertError
                    })?;
            }
        }

        Ok(())
    }

    /// 获取 QR 码的实际尺寸
    pub fn get_size(&self, content: &str) -> AppResult<Size> {
        let qr = QrCode::new(content).map_err(|_| {
            log::error!("Failed to generate QR code");
            AppError::ConvertError
        })?;
        let size = qr.version().width() as u32;

        Ok(Size::new(size * self.scale, size * self.scale))
    }
}

/// 默认 QR 码渲染器配置
impl Default for QrRenderer {
    fn default() -> Self {
        // 默认在屏幕左侧渲染，缩放比例为 2
        Self {
            x: 10,
            y: 10,
            scale: 2,
        }
    }
}
