use embedded_graphics::draw_target::DrawTarget;
use embedded_graphics::primitives::Rectangle;
use epd_waveshare::color::QuadColor;
use epd_waveshare::epd7in5_yrd0750ryf665f60::Display7in5;

use crate::common::display::DisplayData;
use crate::common::error::Result;
use crate::driver::display::{DefaultDisplayDriver, DisplayDriver};

/// 渲染引擎 - 负责管理显示缓冲区和协调渲染
pub struct RenderEngine {
    /// 显示缓冲区
    display_buffer: Display7in5,
    /// 显示驱动引用
    display_driver: DefaultDisplayDriver,
}

impl RenderEngine {
    pub fn new(display_driver: DefaultDisplayDriver) -> Self {
        Self {
            display_buffer: Display7in5::default(),
            // text_styles: TextStyles::new(),
            display_driver,
        }
    }

    /// 渲染完整显示内容
    pub async fn render_full_display<'a>(&mut self, _data: &'a DisplayData<'_>) -> Result<()> {
        log::info!("Rendering full display");

        // 清空显示缓冲区
        self.clear_buffer();

        // 渲染所有组件
        // self.render_time(&data.time).await?;
        // self.render_date(&data.time).await?;
        // self.render_weather(&data.weather).await?;
        // self.render_quote(&data.quote).await?;
        // self.render_status(&data.status).await?;

        // 更新到显示设备
        self.display_driver
            .update_and_display_frame(self.display_buffer.buffer())?;

        log::debug!("Full display rendering completed");
        Ok(())
    }

    /// 渲染部分显示内容
    pub async fn render_partial_display<'a>(
        &mut self,
        _data: &'a DisplayData<'_>,
        area: Rectangle,
    ) -> Result<()> {
        log::debug!("Rendering partial display for area: {:?}", area);

        // 只渲染与刷新区域相交的组件
        // if area.intersects(&LayoutConfig::TIME_REGION) {
        //     self.render_time(&data.time).await?;
        // }

        // if area.intersects(&LayoutConfig::DATE_REGION) {
        //     self.render_date(&data.time).await?;
        // }

        // if area.intersects(&LayoutConfig::WEATHER_REGION) {
        //     self.render_weather(&data.weather).await?;
        // }

        // if area.intersects(&LayoutConfig::QUOTE_REGION) {
        //     self.render_quote(&data.quote).await?;
        // }

        // if area.intersects(&LayoutConfig::STATUS_REGION) {
        //     self.render_status(&data.status).await?;
        // }

        // 更新到显示设备
        self.display_driver
            .update_and_display_frame(self.display_buffer.buffer())?;

        log::debug!("Partial display rendering completed");
        Ok(())
    }

    /// 清空显示缓冲区
    pub fn clear_buffer(&mut self) {
        self.display_buffer.clear(QuadColor::White);
    }

    /// 获取显示缓冲区引用（用于直接绘制）
    pub fn get_buffer(&mut self) -> &mut Display7in5 {
        &mut self.display_buffer
    }
}
