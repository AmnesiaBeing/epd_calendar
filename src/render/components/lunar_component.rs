use alloc::string::ToString;
use embedded_graphics::{
    Drawable,
    prelude::{DrawTarget, Point, Size},
    primitives::Rectangle,
};
use epd_waveshare::color::QuadColor;

use crate::{
    assets::generated_fonts::FontSize,
    common::system_state::LunarData,
    render::{TextRenderer, text_renderer::TextAlignment},
};

const LUNAR_YAER_MONTH_ZODIAC_RECT: Rectangle =
    Rectangle::new(Point::new(10, 135), Size::new(230, 24));
const LUNAR_DAY_RECT: Rectangle = Rectangle::new(Point::new(10, 161), Size::new(80, 40));
const LUNAR_TABOO_RECOMMEND_RECT: Rectangle =
    Rectangle::new(Point::new(100, 135), Size::new(140, 34));
const LUNAR_TABOO_AVOID_RECT: Rectangle = Rectangle::new(Point::new(100, 160), Size::new(140, 34));

impl Drawable for LunarData {
    type Color = QuadColor;
    type Output = ();

    fn draw<D>(&self, target: &mut D) -> Result<Self::Output, D::Error>
    where
        D: DrawTarget<Color = Self::Color>,
    {
        // 1. 绘制农历年月生肖：农历乙巳蛇年冬月
        let lunar_year_month_text = alloc::format!(
            "农历{}{}年{}",
            self.ganzhi.to_string(),
            self.zodiac.to_string(),
            self.day.get_month().to_string()
        );

        // 创建TextRenderer用于绘制农历年月生肖
        let mut year_month_renderer = TextRenderer::new(
            FontSize::Medium,
            Point::new(
                LUNAR_YAER_MONTH_ZODIAC_RECT.top_left.x,
                LUNAR_YAER_MONTH_ZODIAC_RECT.top_left.y,
            ),
        );

        // 计算居中位置
        let year_month_width = year_month_renderer.calculate_text_width(&lunar_year_month_text);
        let year_month_rect_center_x = LUNAR_YAER_MONTH_ZODIAC_RECT.top_left.x
            + (LUNAR_YAER_MONTH_ZODIAC_RECT.size.width as i32) / 2;
        let year_month_start_x = year_month_rect_center_x - (year_month_width as i32) / 2;

        // 调整到居中位置并绘制
        year_month_renderer.move_to(Point::new(
            year_month_start_x,
            LUNAR_YAER_MONTH_ZODIAC_RECT.top_left.y,
        ));
        year_month_renderer.draw_text(target, &lunar_year_month_text)?;

        // 2. 绘制农历日期：初二
        let lunar_day_text = self.day.to_string();

        // 创建TextRenderer用于绘制农历日期
        let mut day_renderer = TextRenderer::new(
            FontSize::Large,
            Point::new(LUNAR_DAY_RECT.top_left.x, LUNAR_DAY_RECT.top_left.y),
        );

        // 计算居中位置
        let day_width = day_renderer.calculate_text_width(&lunar_day_text);
        let day_rect_center_x = LUNAR_DAY_RECT.top_left.x + (LUNAR_DAY_RECT.size.width as i32) / 2;
        let day_start_x = day_rect_center_x - (day_width as i32) / 2;

        // 垂直居中计算
        let day_rect_center_y = LUNAR_DAY_RECT.top_left.y + (LUNAR_DAY_RECT.size.height as i32) / 2;
        let day_start_y = day_rect_center_y - (FontSize::Large.pixel_size() as i32) / 2;

        // 调整到居中位置并绘制
        day_renderer.move_to(Point::new(day_start_x, day_start_y));
        day_renderer.draw_text(target, &lunar_day_text)?;

        // 3. 绘制"宜："+适宜的事件
        // 创建TextRenderer用于绘制宜忌内容
        let mut recommend_renderer = TextRenderer::new(
            FontSize::Small,
            Point::new(
                LUNAR_TABOO_RECOMMEND_RECT.top_left.x,
                LUNAR_TABOO_RECOMMEND_RECT.top_left.y,
            ),
        );

        // 将适宜的事件连接成字符串
        let recommends_text = self
            .day_recommends
            .iter()
            .map(|taboo| taboo.to_string())
            .collect::<alloc::string::String>();

        let small_font_pixel_size = FontSize::Small.pixel_size();

        if !recommends_text.is_empty() {
            // 先绘制"宜："
            recommend_renderer.draw_text(target, "宜：")?;

            // 计算"宜："的宽度
            let colon_width = recommend_renderer.calculate_text_width("宜：") as i32;

            // 计算剩余宽度用于多行显示
            let remaining_width = LUNAR_TABOO_RECOMMEND_RECT.size.width as i32 - colon_width;

            // 记录当前位置
            let current_y = recommend_renderer.current_position().y;

            // 调整位置到"："后面，准备绘制内容
            recommend_renderer.move_to(Point::new(
                LUNAR_TABOO_RECOMMEND_RECT.top_left.x + colon_width,
                current_y - small_font_pixel_size as i32, // 回到上一行
            ));

            // 绘制多行内容（自动换行）
            recommend_renderer.draw_text_multiline(
                target,
                &recommends_text,
                remaining_width,
                TextAlignment::Left,
            )?;
        }

        // 4. 绘制"忌："+避免的事件
        // 创建TextRenderer用于绘制忌的内容
        let mut avoid_renderer = TextRenderer::new(
            FontSize::Small,
            Point::new(
                LUNAR_TABOO_AVOID_RECT.top_left.x,
                LUNAR_TABOO_AVOID_RECT.top_left.y,
            ),
        );

        // 将避免的事件连接成字符串
        let avoids_text = self
            .day_avoids
            .iter()
            .map(|taboo| taboo.to_string())
            .collect::<alloc::string::String>();

        if !avoids_text.is_empty() {
            // 先绘制"忌："
            avoid_renderer.draw_text(target, "忌：")?;

            // 计算"忌："的宽度
            let colon_width = avoid_renderer.calculate_text_width("忌：") as i32;

            // 计算剩余宽度用于多行显示
            let remaining_width = LUNAR_TABOO_AVOID_RECT.size.width as i32 - colon_width;

            // 记录当前位置
            let current_y = avoid_renderer.current_position().y;

            // 调整位置到"："后面，准备绘制内容
            avoid_renderer.move_to(Point::new(
                LUNAR_TABOO_AVOID_RECT.top_left.x + colon_width,
                current_y - small_font_pixel_size as i32, // 回到上一行
            ));

            // 绘制多行内容（自动换行）
            avoid_renderer.draw_text_multiline(
                target,
                &avoids_text,
                remaining_width,
                TextAlignment::Left,
            )?;
        }

        Ok(())
    }
}
