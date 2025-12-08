use alloc::vec;
use alloc::{
    string::{String, ToString},
    vec::Vec,
};
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

// 农历显示总容器：0,180 到 240,390（宽240px，高210px）
const LUNAR_CONTAINER: Rectangle = Rectangle::new(Point::new(0, 180), Size::new(300, 210));
// 排版间距配置（可根据视觉效果微调）
const SPACING_YEAR_MONTH_DAY: u32 = 8; // 年月与日期的间距
const SPACING_DAY_TABOO: u32 = 8; // 日期与宜忌的间距
const SPACING_RECOMMEND_AVOID: u32 = 4; // 宜与忌的间距
const TABOO_LABEL_WIDTH_OFFSET: i32 = 25; // “宜：”/“忌：”的固定宽度（Small字体）

impl Drawable for LunarData {
    type Color = QuadColor;
    type Output = ();

    fn draw<D>(&self, target: &mut D) -> Result<Self::Output, D::Error>
    where
        D: DrawTarget<Color = Self::Color>,
    {
        // ========== 1. 准备文本数据 ==========
        // 农历年月文本："农历<干支><生肖>年<农历月>"（8字）
        let lunar_year_month_text = alloc::format!(
            "农历{}{}年{}",
            self.ganzhi.to_string(),
            self.zodiac.to_string(),
            self.day.get_lunar_month().to_string()
        );

        // 农历日期文本：优先显示节日/节气，否则显示日期
        let lunar_day_text = if !self.festival.is_none() {
            self.festival.clone().unwrap().to_string()
        } else if !self.jieqi.is_none() {
            self.jieqi.clone().unwrap().to_string()
        } else {
            self.day.to_string()
        };

        // 适宜/忌讳文本（用空格分隔，避免连在一起）
        let recommends_text = self
            .day_recommends
            .iter()
            .map(|t| t.to_string())
            .collect::<Vec<_>>()
            .join(" ");
        let avoids_text = self
            .day_avoids
            .iter()
            .map(|t| t.to_string())
            .collect::<Vec<_>>()
            .join(" ");

        // ========== 2. 初始化渲染器 ==========
        let mut year_month_renderer = TextRenderer::new(FontSize::Medium, Point::zero());
        let mut day_renderer = TextRenderer::new(FontSize::Large, Point::zero());
        let mut taboo_renderer = TextRenderer::new(FontSize::Small, Point::zero());

        // ========== 3. 绘制农历年月（顶部+水平居中） ==========
        // 计算年月文本的水平居中位置
        let year_month_height = year_month_renderer.get_line_height() as i32;
        let year_month_width = year_month_renderer.calculate_text_width(&lunar_year_month_text);
        let year_month_x =
            LUNAR_CONTAINER.top_left.x + (LUNAR_CONTAINER.size.width as i32 - year_month_width) / 2;
        let year_month_y = LUNAR_CONTAINER.top_left.y + year_month_height + 5;
        let year_month_pos = Point::new(year_month_x, year_month_y);

        // 绘制年月文本
        year_month_renderer.move_to(year_month_pos);
        year_month_renderer.draw_single_line(target, &lunar_year_month_text)?;

        // ========== 4. 绘制农历日期（100px区域内居中） ==========
        // 日期区位置：年月底部 + 间距，高度100px，宽度与容器一致
        let day_area_top = year_month_y + year_month_height as i32 + SPACING_YEAR_MONTH_DAY as i32;
        let day_area_rect = Rectangle::new(
            Point::new(LUNAR_CONTAINER.top_left.x, day_area_top),
            Size::new(LUNAR_CONTAINER.size.width, 100),
        );

        // 计算日期文本的居中位置
        let day_width = day_renderer.calculate_text_width(&lunar_day_text);
        let day_x = day_area_rect.top_left.x + (day_area_rect.size.width as i32 - day_width) / 2;
        let day_y = day_area_rect.top_left.y
            + (day_area_rect.size.height as i32 - day_renderer.get_line_height() as i32) / 2;
        let day_pos = Point::new(day_x, day_y);

        // 绘制日期文本
        day_renderer.move_to(day_pos);
        day_renderer.draw_single_line(target, &lunar_day_text)?;

        // ========== 5. 绘制适宜/忌讳内容（剩余区域垂直居中） ==========
        // 计算宜忌区位置：日期区底部 + 间距
        let taboo_area_top = day_y + SPACING_DAY_TABOO as i32;
        let taboo_area_height =
            LUNAR_CONTAINER.size.height as i32 - (taboo_area_top - LUNAR_CONTAINER.top_left.y) - 5;
        let taboo_area_rect = Rectangle::new(
            Point::new(LUNAR_CONTAINER.top_left.x + 5, taboo_area_top), // 左侧留5px边距
            Size::new(LUNAR_CONTAINER.size.width - 10, taboo_area_height as u32), // 左右各5px边距
        );

        // 准备宜忌文本块
        let mut taboo_lines = Vec::new();
        let mut total_taboo_height = 0;

        // 处理“宜：”内容
        if !recommends_text.is_empty() {
            let recommend_label = "宜：";
            let recommend_label_width = taboo_renderer.calculate_text_width(recommend_label);
            let recommend_content_width = taboo_area_rect.size.width as i32 - recommend_label_width;

            // 计算“宜：”行的高度
            let recommend_lines = calculate_text_lines(
                &mut taboo_renderer,
                &recommends_text,
                recommend_content_width,
            );
            let recommend_line_height = taboo_renderer.get_line_height() as i32;
            let recommend_total_height = recommend_lines.len() as i32 * recommend_line_height;
            total_taboo_height += recommend_total_height + SPACING_RECOMMEND_AVOID as i32; // 加间距

            taboo_lines.push((recommend_label, recommends_text, recommend_content_width));
        }

        // 处理“忌：”内容
        if !avoids_text.is_empty() {
            let avoid_label = "忌：";
            let avoid_label_width = taboo_renderer.calculate_text_width(avoid_label);
            let avoid_content_width = taboo_area_rect.size.width as i32 - avoid_label_width;

            // 计算“忌：”行的高度
            let avoid_lines =
                calculate_text_lines(&mut taboo_renderer, &avoids_text, avoid_content_width);
            let avoid_line_height = taboo_renderer.get_line_height() as i32;
            let avoid_total_height = avoid_lines.len() as i32 * avoid_line_height;
            total_taboo_height += avoid_total_height;

            taboo_lines.push((avoid_label, avoids_text, avoid_content_width));
        }

        // 计算宜忌区的垂直居中偏移
        let taboo_start_y = if total_taboo_height > 0 && taboo_area_height > total_taboo_height {
            taboo_area_rect.top_left.y + (taboo_area_height - total_taboo_height) / 2
        } else {
            taboo_area_rect.top_left.y
        };
        let mut current_taboo_y = taboo_start_y;

        // 绘制宜忌内容
        for (label, content, content_width) in taboo_lines {
            // 绘制标签（“宜：”/“忌：”）
            taboo_renderer.move_to(Point::new(taboo_area_rect.top_left.x, current_taboo_y));
            taboo_renderer.draw_single_line(target, label)?;

            // 计算内容绘制起始位置（标签宽度后）
            let content_x = taboo_area_rect.top_left.x + taboo_renderer.calculate_text_width(label);
            taboo_renderer.move_to(Point::new(content_x, current_taboo_y));

            // 绘制内容（自动换行，换行后对齐内容起始位置）
            taboo_renderer.draw_text_multiline(
                target,
                &content,
                content_width,
                TextAlignment::Left,
            )?;

            // 更新下一行Y坐标
            let content_line_count =
                calculate_text_lines(&mut taboo_renderer, &content, content_width).len() as i32;
            current_taboo_y += content_line_count * taboo_renderer.get_line_height() as i32
                + SPACING_RECOMMEND_AVOID as i32;
        }

        Ok(())
    }
}

/// 辅助函数：计算文本在指定宽度下的行数（按单词拆分，避免截断）
fn calculate_text_lines(renderer: &mut TextRenderer, text: &str, max_width: i32) -> Vec<String> {
    if text.is_empty() || max_width <= 0 {
        return Vec::new();
    }

    // 按空格拆分单词
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.is_empty() {
        return vec![text.to_string()];
    }

    let mut lines = Vec::new();
    let mut current_line = words[0].to_string();
    let mut current_width = renderer.calculate_text_width(&current_line);

    for &word in &words[1..] {
        let word_width = renderer.calculate_text_width(word);
        let space_width = renderer.calculate_text_width(" ");
        let new_width = current_width + space_width + word_width;

        if new_width <= max_width {
            current_line.push(' ');
            current_line.push_str(word);
            current_width = new_width;
        } else {
            lines.push(current_line);
            current_line = word.to_string();
            current_width = word_width;
        }
    }

    lines.push(current_line);
    lines
}
