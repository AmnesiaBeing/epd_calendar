use alloc::{format, string::ToString};
use embedded_graphics::{
    Drawable,
    prelude::{DrawTarget, Point},
};
use epd_waveshare::color::QuadColor;

use crate::{
    assets::generated_fonts::FontSize,
    common::system_state::DateData,
    render::{TextRenderer, components::time_component::TIME_RECT},
};

// 屏幕尺寸常量
const SCREEN_WIDTH: i32 = 800;

// 排版间距配置
const SPACING_TIME_HOLIDAY: u32 = 20; // 时间和节假日之间的间距

impl Drawable for DateData {
    type Color = QuadColor;

    type Output = ();

    fn draw<D>(&self, target: &mut D) -> Result<Self::Output, D::Error>
    where
        D: DrawTarget<Color = Self::Color>,
    {
        // 1. 准备基础文本数据
        let date_week_str = format!(
            "{}-{}-{} 星期{}",
            self.day.get_year(),
            self.day.get_month(),
            self.day.get_day(),
            self.week.to_string()
        );

        // 2. 初始化Large字体渲染器（用于日期和星期）
        let mut large_renderer = TextRenderer::new(
            FontSize::Large,
            Point::zero(), // 临时位置，后续会动态计算
        );

        // 3. 计算日期的基线位置（时间正下方 + 居中）
        // 3.1 计算日期基线Y坐标：时间区域底部 + 间距 + 字体行高适配
        let time_area_bottom_y = TIME_RECT.top_left.y + TIME_RECT.size.height as i32;
        let date_baseline_y = time_area_bottom_y + large_renderer.get_line_height() as i32 - 5; // 稍微偏移一下，中间间距太大了

        // 3.2 计算日期基线X坐标（屏幕居中）
        let date_width = large_renderer.calculate_text_width(&date_week_str);
        let date_baseline_x = (SCREEN_WIDTH - date_width) / 2;
        let date_baseline = Point::new(date_baseline_x, date_baseline_y);

        // 4. 绘制日期（居中）
        large_renderer.move_to(date_baseline);
        large_renderer.draw_single_line(target, &date_week_str)?;

        // 7. 处理节假日绘制（Medium字体，底部与日期平齐）
        if let Some(holiday) = &self.holiday {
            let holiday_str = holiday.to_string();

            // 7.1 初始化Medium字体渲染器
            let mut medium_renderer = TextRenderer::new(
                FontSize::Medium,
                Point::zero(), // 临时位置
            );

            // 7.2 计算节假日的X坐标（时间区域右侧 + 间距）
            let holiday_baseline_x =
                TIME_RECT.top_left.x + TIME_RECT.size.width as i32 + SPACING_TIME_HOLIDAY as i32;

            // 7.3 计算节假日的Y坐标（底部与日期平齐）
            // 获取日期字符的度量参数（取第一个字符作为基准）
            let date_metrics = large_renderer
                .get_font_size()
                .get_glyph_metrics(date_week_str.chars().next().unwrap())
                .unwrap_or_else(|| {
                    // 降级处理：使用默认度量参数
                    large_renderer
                        .get_font_size()
                        .get_glyph_metrics('0')
                        .unwrap()
                });

            // 计算日期的底部Y坐标
            let date_bottom_y =
                date_baseline_y - date_metrics.bearing_x + date_metrics.height as i32;

            // 获取节假日字符的度量参数（取第一个字符作为基准）
            let holiday_metrics = medium_renderer
                .get_font_size()
                .get_glyph_metrics(holiday_str.chars().next().unwrap())
                .unwrap_or_else(|| {
                    // 降级处理：使用默认度量参数
                    medium_renderer
                        .get_font_size()
                        .get_glyph_metrics('节')
                        .unwrap()
                });

            // 计算节假日的基线Y坐标（保证底部对齐）
            let holiday_baseline_y =
                date_bottom_y - holiday_metrics.height as i32 + holiday_metrics.bearing_y;

            // 7.4 绘制节假日文本
            let holiday_baseline = Point::new(holiday_baseline_x, holiday_baseline_y);
            medium_renderer.move_to(holiday_baseline);
            medium_renderer.draw_single_line(target, &holiday_str)?;
        }

        Ok(())
    }
}
