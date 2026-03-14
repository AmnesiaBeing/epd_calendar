//! 布局渲染引擎
//! 解析布局定义并渲染到 framebuffer
//!
//! 注意：此模块设计为 no_std 兼容，JSON 解析在外部完成

extern crate alloc;

use alloc::boxed::Box;
use alloc::format;
use alloc::string::String;
use core::str::FromStr;

use hash32::{BuildHasherDefault, FnvHasher};
use heapless::IndexMap;

use super::framebuffer::{Color, Framebuffer};
use super::icon::IconRenderer;
use super::text::TextRenderer;
use lxx_calendar_common::SystemResult;
use lxx_calendar_common::layout::*;

/// 布局渲染引擎
///
/// 此引擎接收已解析的布局定义和数据进行渲染
/// JSON 解析应在外部完成（在 std 环境下），然后传递解析后的数据
pub struct LayoutEngine {
    text_renderer: TextRenderer,
    icon_renderer: IconRenderer,
    current_y: u16,
    screen_width: u16,
    screen_height: u16,
}

impl LayoutEngine {
    /// 创建新的布局渲染引擎
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            text_renderer: TextRenderer::new(),
            icon_renderer: IconRenderer::new(),
            current_y: 0,
            screen_width: width,
            screen_height: height,
        }
    }

    /// 渲染布局定义
    ///
    /// # 参数
    /// * `framebuffer` - 帧缓冲区
    /// * `layout` - 已解析的布局定义
    /// * `data` - 渲染数据（使用 heapless::Value 或自定义数据结构）
    pub fn render<const SIZE: usize>(
        &mut self,
        framebuffer: &mut Framebuffer<SIZE>,
        layout: &LayoutDefinition,
        data: &LayoutData,
    ) -> SystemResult<()> {
        // 渲染状态栏
        if let Some(_status_bar) = &layout.status_bar {
            self.render_status_bar(framebuffer)?;
        }

        // 渲染主体内容
        self.current_y = layout.status_bar.is_some().then(|| 20u16).unwrap_or(0);
        self.render_blocks(framebuffer, &layout.body, data)?;

        // 渲染页脚
        if let Some(footer) = &layout.footer {
            self.render_footer(framebuffer, footer, data)?;
        }

        Ok(())
    }

    /// 渲染状态栏
    fn render_status_bar<const SIZE: usize>(
        &mut self,
        framebuffer: &mut Framebuffer<SIZE>,
    ) -> SystemResult<()> {
        // 绘制状态栏分隔线
        let y = 18;
        framebuffer.draw_horizontal_line(0, y, self.screen_width, Color::Black)?;
        Ok(())
    }

    /// 渲染页脚
    fn render_footer<const SIZE: usize>(
        &mut self,
        framebuffer: &mut Framebuffer<SIZE>,
        footer: &FooterConfig,
        data: &LayoutData,
    ) -> SystemResult<()> {
        // 计算页脚位置（屏幕底部）
        let footer_height = 20u16;
        let footer_y = self.screen_height.saturating_sub(footer_height);

        // 绘制页脚分隔线
        framebuffer.draw_horizontal_line(0, footer_y, self.screen_width, Color::Black)?;

        // 渲染页脚标签
        if let Some(label) = &footer.label {
            let x = 10u16;
            let y = footer_y + 4;
            self.text_renderer
                .render(framebuffer, x, y, label.as_str())?;
        }

        // 渲染归属信息
        if let Some(template) = &footer.attribution_template {
            let text = Self::apply_template(template, data);
            let x = self.screen_width.saturating_sub(text.len() as u16 * 8 + 10);
            let y = footer_y + 4;
            self.text_renderer.render(framebuffer, x, y, &text)?;
        }

        self.current_y = footer_y;
        Ok(())
    }

    /// 渲染块列表
    fn render_blocks<const SIZE: usize>(
        &mut self,
        framebuffer: &mut Framebuffer<SIZE>,
        blocks: &[Block],
        data: &LayoutData,
    ) -> SystemResult<()> {
        for block in blocks {
            self.render_block(framebuffer, block, data)?;
        }
        Ok(())
    }

    /// 渲染单个块
    fn render_block<const SIZE: usize>(
        &mut self,
        framebuffer: &mut Framebuffer<SIZE>,
        block: &Block,
        data: &LayoutData,
    ) -> SystemResult<()> {
        match block {
            Block::Text(text_block) => self.render_text_block(framebuffer, text_block, data),
            Block::Separator(separator_block) => {
                self.render_separator_block(framebuffer, separator_block)
            }
            Block::Spacer(spacer_block) => self.render_spacer_block(spacer_block),
            Block::BigNumber(big_number_block) => {
                self.render_big_number_block(framebuffer, big_number_block, data)
            }
            Block::IconText(icon_text_block) => {
                self.render_icon_text_block(framebuffer, icon_text_block, data)
            }
            Block::TwoColumn(two_column_block) => {
                self.render_two_column_block(framebuffer, two_column_block, data)
            }
            Block::WeatherIconText(weather_icon_text_block) => {
                self.render_weather_icon_text_block(framebuffer, weather_icon_text_block, data)
            }
            Block::ProgressBar(progress_bar_block) => {
                self.render_progress_bar_block(framebuffer, progress_bar_block, data)
            }
            Block::Image(image_block) => self.render_image_block(framebuffer, image_block, data),
            Block::CenteredText(centered_text_block) => {
                self.render_centered_text_block(framebuffer, centered_text_block, data)
            }
            Block::VerticalStack(vertical_stack_block) => {
                self.render_vertical_stack_block(framebuffer, vertical_stack_block, data)
            }
            Block::Conditional(conditional_block) => {
                self.render_conditional_block(framebuffer, conditional_block, data)
            }
            Block::Section(section_block) => {
                self.render_section_block(framebuffer, section_block, data)
            }
            Block::List(list_block) => self.render_list_block(framebuffer, list_block, data),
            Block::IconList(icon_list_block) => {
                self.render_icon_list_block(framebuffer, icon_list_block, data)
            }
            Block::KeyValue(key_value_block) => {
                self.render_key_value_block(framebuffer, key_value_block, data)
            }
            Block::Group(group_block) => self.render_group_block(framebuffer, group_block, data),
            Block::ForecastCards(forecast_cards_block) => {
                self.render_forecast_cards_block(framebuffer, forecast_cards_block, data)
            }
        }
    }

    /// 渲染文本块
    fn render_text_block<const SIZE: usize>(
        &mut self,
        framebuffer: &mut Framebuffer<SIZE>,
        block: &TextBlock,
        data: &LayoutData,
    ) -> SystemResult<()> {
        let text = Self::get_text_content(data, block.field.as_deref(), block.template.as_deref());

        let x = block.margin_x;
        let y = self.current_y + 4;

        self.text_renderer.render(framebuffer, x, y, &text)?;

        // 更新 Y 位置
        self.current_y += block.font_size + 8;

        Ok(())
    }

    /// 渲染居中文本块
    fn render_centered_text_block<const SIZE: usize>(
        &mut self,
        framebuffer: &mut Framebuffer<SIZE>,
        block: &CenteredTextBlock,
        data: &LayoutData,
    ) -> SystemResult<()> {
        let text = data.get_string(&block.field).unwrap_or_default();

        // 估算文本宽度
        let text_width = (text.len() as u16) * (block.font_size / 2 + 2);
        let start_x = if self.screen_width > text_width {
            (self.screen_width - text_width) / 2
        } else {
            0
        };

        let y = self.current_y + 4;
        self.text_renderer.render(framebuffer, start_x, y, &text)?;

        self.current_y += block.font_size + block.line_spacing;

        Ok(())
    }

    /// 渲染分隔线块
    fn render_separator_block<const SIZE: usize>(
        &mut self,
        framebuffer: &mut Framebuffer<SIZE>,
        block: &SeparatorBlock,
    ) -> SystemResult<()> {
        let y = self.current_y + 4;
        let margin_x = block.margin_x;

        let width = block
            .width
            .unwrap_or_else(|| self.screen_width.saturating_sub(margin_x * 2));

        match block.style {
            SeparatorStyle::Solid => {
                framebuffer.draw_horizontal_line(margin_x, y, width, Color::Black)?;
            }
            SeparatorStyle::Dashed => {
                // 绘制虚线
                let dash_len = 4u16;
                let gap_len = 2u16;
                let mut x = margin_x;
                while x < margin_x + width {
                    let dash_width = dash_len.min(margin_x + width - x);
                    framebuffer.draw_horizontal_line(x, y, dash_width, Color::Black)?;
                    x += dash_len + gap_len;
                }
            }
            SeparatorStyle::Short => {
                let short_width = width.min(100);
                let start_x = margin_x + (width - short_width) / 2;
                framebuffer.draw_horizontal_line(start_x, y, short_width, Color::Black)?;
            }
        }

        self.current_y += 8;

        Ok(())
    }

    /// 渲染间距块
    fn render_spacer_block(&mut self, block: &SpacerBlock) -> SystemResult<()> {
        self.current_y += block.height;
        Ok(())
    }

    /// 渲染大号数字块
    fn render_big_number_block<const SIZE: usize>(
        &mut self,
        framebuffer: &mut Framebuffer<SIZE>,
        block: &BigNumberBlock,
        data: &LayoutData,
    ) -> SystemResult<()> {
        let text = data.get_string(&block.field).unwrap_or_default();

        let x = match block.align {
            TextAlign::Left => block.margin_x,
            TextAlign::Center => {
                let text_width = (text.len() as u16) * (block.font_size / 2 + 2);
                if self.screen_width > text_width {
                    (self.screen_width - text_width) / 2
                } else {
                    0
                }
            }
            TextAlign::Right => {
                let text_width = (text.len() as u16) * (block.font_size / 2 + 2);
                self.screen_width
                    .saturating_sub(text_width + block.margin_x)
            }
        };

        let y = self.current_y + 4;

        // 使用大号字体渲染
        self.text_renderer
            .render_large_with_size(framebuffer, x, y, &text, block.font_size)?;

        // 渲染单位
        if let Some(unit) = &block.unit {
            let unit_x = x + (text.len() as u16) * (block.font_size / 2 + 2) + 4;
            let unit_y = y + block.font_size / 3;
            self.text_renderer.render_with_size(
                framebuffer,
                unit_x,
                unit_y,
                unit,
                block.font_size / 2,
            )?;
        }

        self.current_y += block.font_size + 8;

        Ok(())
    }

    /// 渲染图标 + 文本块
    fn render_icon_text_block<const SIZE: usize>(
        &mut self,
        framebuffer: &mut Framebuffer<SIZE>,
        block: &IconTextBlock,
        data: &LayoutData,
    ) -> SystemResult<()> {
        let text = Self::get_text_content(data, block.field.as_deref(), block.text.as_deref());

        let mut x = block.margin_x;
        let y = self.current_y + 4;

        // 渲染图标（如果有）
        if let Some(icon_name) = &block.icon {
            // TODO: 使用生成的图标数据渲染图标
            // 暂时绘制一个占位符矩形
            let icon_size = block.icon_size;
            framebuffer.draw_rectangle(x, y, icon_size, icon_size, Color::Black)?;
            x += icon_size + 4;
        }

        // 渲染文本
        self.text_renderer
            .render_with_size(framebuffer, x, y, &text, block.font_size)?;

        self.current_y += block.font_size.max(block.icon_size) + 4;

        Ok(())
    }

    /// 渲染天气图标 + 文本块
    fn render_weather_icon_text_block<const SIZE: usize>(
        &mut self,
        framebuffer: &mut Framebuffer<SIZE>,
        block: &WeatherIconTextBlock,
        data: &LayoutData,
    ) -> SystemResult<()> {
        let code = data.get_string(&block.code_field).unwrap_or_default();
        let text = data.get_string(&block.field).unwrap_or_default();

        let mut x = block.margin_x;
        let y = self.current_y + 4;

        // 渲染天气图标（使用图标代码）
        // 默认白天，TODO: 从系统时间获取 is_day
        let is_day = true;
        let icon_code = if code.is_empty() {
            // 如果没有代码，尝试从文本推断
            if text.contains("晴") {
                if is_day { "100" } else { "150" }
            } else if text.contains("雨") {
                "306"
            } else if text.contains("雪") {
                "400"
            } else if text.contains("雷") {
                "302"
            } else if text.contains("雾") {
                "501"
            } else {
                "104" // 默认阴天
            }
        } else {
            code.as_str()
        };

        self.icon_renderer
            .render_weather_icon_by_code(framebuffer, x, y, icon_code)?;
        x += block.icon_size + 4;

        // 渲染文本
        self.text_renderer
            .render_with_size(framebuffer, x, y, &text, block.font_size)?;

        self.current_y += block.font_size.max(block.icon_size) + 4;

        Ok(())
    }

    /// 渲染两列布局块
    fn render_two_column_block<const SIZE: usize>(
        &mut self,
        framebuffer: &mut Framebuffer<SIZE>,
        block: &TwoColumnBlock,
        data: &LayoutData,
    ) -> SystemResult<()> {
        let start_y = self.current_y;

        // 渲染左列
        let left_x = block.left_x;
        let mut left_engine = LayoutEngine::new(block.left_width, self.screen_height);
        left_engine.current_y = start_y;
        left_engine.render_blocks(framebuffer, &block.left, data)?;

        // 渲染右列
        let right_x = left_x + block.left_width + block.gap;
        let right_width = self.screen_width.saturating_sub(right_x);
        let mut right_engine = LayoutEngine::new(right_width, self.screen_height);
        right_engine.current_y = start_y;
        right_engine.render_blocks(framebuffer, &block.right, data)?;

        // 更新 Y 位置为两列中较高的那个
        self.current_y = left_engine.current_y.max(right_engine.current_y);

        Ok(())
    }

    /// 渲染进度条块
    fn render_progress_bar_block<const SIZE: usize>(
        &mut self,
        framebuffer: &mut Framebuffer<SIZE>,
        block: &ProgressBarBlock,
        data: &LayoutData,
    ) -> SystemResult<()> {
        let current = data.get_f64(&block.field).unwrap_or(0.0);
        let max = data.get_f64(&block.max_field).unwrap_or(100.0);

        let ratio = if max > 0.0 {
            (current / max).min(1.0)
        } else {
            0.0
        };

        let x = block.margin_x;
        let y = self.current_y + 4;
        let width = block.width;
        let height = block.height;

        // 绘制背景
        framebuffer.draw_rectangle(x, y, width, height, Color::White)?;
        framebuffer.draw_rectangle(x, y, width, height, Color::Black)?;

        // 绘制进度
        let fill_width = ((width as f64) * ratio) as u16;
        if fill_width > 0 {
            framebuffer.fill_rectangle(
                x + 1,
                y + 1,
                fill_width.saturating_sub(2),
                height.saturating_sub(2),
                Color::White,
            )?;
        }

        self.current_y += height + 4;

        Ok(())
    }

    /// 渲染图片块
    fn render_image_block<const SIZE: usize>(
        &mut self,
        framebuffer: &mut Framebuffer<SIZE>,
        block: &ImageBlock,
        data: &LayoutData,
    ) -> SystemResult<()> {
        // TODO: 渲染图片数据
        // 暂时绘制一个占位符矩形
        let x = block.x;
        let y = self.current_y + block.y;
        let width = block.width.unwrap_or(100);
        let height = block.height.unwrap_or(100);

        framebuffer.draw_rectangle(x, y, width, height, Color::Black)?;

        self.current_y += height + 4;

        Ok(())
    }

    /// 渲染垂直堆叠块
    fn render_vertical_stack_block<const SIZE: usize>(
        &mut self,
        framebuffer: &mut Framebuffer<SIZE>,
        block: &VerticalStackBlock,
        data: &LayoutData,
    ) -> SystemResult<()> {
        for (i, child) in block.children.iter().enumerate() {
            if i > 0 && block.spacing > 0 {
                self.render_spacer_block(&SpacerBlock {
                    height: block.spacing,
                })?;
            }
            self.render_block(framebuffer, child, data)?;
        }
        Ok(())
    }

    /// 渲染条件块
    fn render_conditional_block<const SIZE: usize>(
        &mut self,
        framebuffer: &mut Framebuffer<SIZE>,
        block: &ConditionalBlock,
        data: &LayoutData,
    ) -> SystemResult<()> {
        // TODO: 实现条件块评估
        // 暂时跳过条件块，直接渲染回退块
        if let Some(fallback) = &block.fallback_children {
            self.render_blocks(framebuffer, fallback, data)?;
        }
        Ok(())
    }

    /// 渲染区块
    fn render_section_block<const SIZE: usize>(
        &mut self,
        framebuffer: &mut Framebuffer<SIZE>,
        block: &SectionBlock,
        data: &LayoutData,
    ) -> SystemResult<()> {
        // 渲染标题
        let y = self.current_y + 4;
        self.text_renderer.render_with_size(
            framebuffer,
            10,
            y,
            &block.title,
            block.title_font_size,
        )?;

        self.current_y += block.title_font_size + 8;

        // 渲染子块
        self.render_blocks(framebuffer, &block.children, data)?;

        Ok(())
    }

    /// 渲染列表块
    fn render_list_block<const SIZE: usize>(
        &mut self,
        framebuffer: &mut Framebuffer<SIZE>,
        block: &ListBlock,
        data: &LayoutData,
    ) -> SystemResult<()> {
        if let Some(items) = data.get_array(&block.field) {
            for (i, item) in items.iter().enumerate().take(block.max_items as usize) {
                let mut text = String::new();

                if block.numbered {
                    text.push_str(&format!("{}. ", i + 1));
                }

                if let Some(template) = &block.item_template {
                    text.push_str(&Self::apply_template_to_item(template, item));
                } else {
                    // 简单转换为字符串
                    text.push_str("[item]");
                }

                let x = block.margin_x;
                let y = self.current_y + 4;
                self.text_renderer.render(framebuffer, x, y, &text)?;

                self.current_y += block.font_size + block.item_spacing;
            }
        }

        Ok(())
    }

    /// 渲染图标列表块
    fn render_icon_list_block<const SIZE: usize>(
        &mut self,
        framebuffer: &mut Framebuffer<SIZE>,
        block: &IconListBlock,
        data: &LayoutData,
    ) -> SystemResult<()> {
        if let Some(items) = data.get_array(&block.field) {
            for item in items.iter().take(block.max_items as usize) {
                let mut x = 10u16;
                let y = self.current_y + 4;

                // 渲染图标（如果有）
                if let Some(icon_field) = &block.icon_field {
                    let icon_name = item.get_string(icon_field).unwrap_or_default();
                    // TODO: 渲染图标
                    framebuffer.draw_rectangle(x, y, 16, 16, Color::Black)?;
                    x += 20;
                }

                // 渲染文本（如果有）
                if let Some(text_field) = &block.text_field {
                    let text = item.get_string(text_field).unwrap_or_default();
                    self.text_renderer.render(framebuffer, x, y, &text)?;
                }

                self.current_y += 20;
            }
        }

        Ok(())
    }

    /// 渲染键值对块
    fn render_key_value_block<const SIZE: usize>(
        &mut self,
        framebuffer: &mut Framebuffer<SIZE>,
        block: &KeyValueBlock,
        data: &LayoutData,
    ) -> SystemResult<()> {
        let value = data.get_string(&block.field).unwrap_or_default();
        let label = block.label.as_deref().unwrap_or(&block.field);

        let text = format!("{}: {}", label, value);

        let x = 10u16;
        let y = self.current_y + 4;
        self.text_renderer
            .render_with_size(framebuffer, x, y, &text, block.font_size)?;

        self.current_y += block.font_size + 4;

        Ok(())
    }

    /// 渲染组块
    fn render_group_block<const SIZE: usize>(
        &mut self,
        framebuffer: &mut Framebuffer<SIZE>,
        block: &GroupBlock,
        data: &LayoutData,
    ) -> SystemResult<()> {
        // 渲染组标题
        let y = self.current_y + 4;
        self.text_renderer
            .render(framebuffer, 10, y, &block.title)?;
        self.current_y += 20;

        // 渲染子块
        self.render_blocks(framebuffer, &block.children, data)?;

        Ok(())
    }

    /// 渲染预报卡片块
    fn render_forecast_cards_block<const SIZE: usize>(
        &mut self,
        framebuffer: &mut Framebuffer<SIZE>,
        block: &ForecastCardsBlock,
        data: &LayoutData,
    ) -> SystemResult<()> {
        if let Some(items) = data.get_array(&block.field) {
            let mut x = block.margin_x.max(0) as u16;
            let y = self.current_y + 4;
            let card_width = 60u16;
            let card_height = 80u16;

            for item in items.iter().take(block.max_items as usize) {
                // 绘制卡片边框
                framebuffer.draw_rectangle(x, y, card_width, card_height, Color::Black)?;

                // 渲染卡片内容
                let icon_x = x + (card_width - block.icon_size) / 2;
                let icon_y = y + 4;

                // TODO: 渲染天气图标
                framebuffer.draw_rectangle(
                    icon_x,
                    icon_y,
                    block.icon_size,
                    block.icon_size,
                    Color::Black,
                )?;

                // 渲染温度等信息
                if let Some(temp) = item.get_string("temp") {
                    let temp_x = x + (card_width - temp.len() as u16 * 8) / 2;
                    let temp_y = y + block.icon_size + 8;
                    self.text_renderer
                        .render_with_size(framebuffer, temp_x, temp_y, &temp, 14)?;
                }

                x += card_width + block.gap;
            }

            self.current_y += card_height + 4;
        }

        Ok(())
    }

    /// 从数据中获取文本内容
    fn get_text_content(
        data: &LayoutData,
        field: Option<&str>,
        template: Option<&str>,
    ) -> alloc::string::String {
        if let Some(tpl) = template {
            Self::apply_template(tpl, data)
        } else if let Some(f) = field {
            data.get_string(f)
                .map(|s| s.as_str().into())
                .unwrap_or_default()
        } else {
            alloc::string::String::new()
        }
    }

    /// 应用模板到数据
    fn apply_template(template: &str, data: &LayoutData) -> alloc::string::String {
        let mut result = alloc::string::String::from(template);

        // 简单模板替换：{field_name} -> value
        for field in data.get_fields() {
            if let Some(value) = data.get_string(field) {
                let value_str = value.as_str();
                result = result.replace(&format!("{{{}}}", field), value_str);
            }
        }

        result
    }

    /// 应用模板到列表项
    fn apply_template_to_item(template: &str, item: &LayoutData) -> alloc::string::String {
        let mut result = alloc::string::String::from(template);

        for field in item.get_fields() {
            if let Some(value) = item.get_string(field) {
                let value_str = value.as_str();
                result = result.replace(&format!("{{{}}}", field), value_str);
            }
        }

        result
    }

    /// 评估条件
    fn evaluate_condition(
        op: &lxx_calendar_common::layout::ConditionOp,
        field_value: Option<&LayoutData>,
        compare_value: Option<&LayoutValue>,
    ) -> bool {
        match op {
            lxx_calendar_common::layout::ConditionOp::Eq => {
                if let (Some(fv), Some(cv)) = (field_value, compare_value) {
                    return fv == cv;
                }
                false
            }
            lxx_calendar_common::layout::ConditionOp::Gt => {
                if let (Some(fv), Some(cv)) = (field_value, compare_value) {
                    return fv
                        .partial_cmp(cv)
                        .map(|o| o == core::cmp::Ordering::Greater)
                        .unwrap_or(false);
                }
                false
            }
            lxx_calendar_common::layout::ConditionOp::Lt => {
                if let (Some(fv), Some(cv)) = (field_value, compare_value) {
                    return fv
                        .partial_cmp(cv)
                        .map(|o| o == core::cmp::Ordering::Less)
                        .unwrap_or(false);
                }
                false
            }
            lxx_calendar_common::layout::ConditionOp::Exists => field_value.is_some(),
            // 其他操作符可根据需要添加
            _ => false,
        }
    }

    /// 获取当前 Y 位置
    pub fn current_y(&self) -> u16 {
        self.current_y
    }

    /// 设置当前 Y 位置
    pub fn set_current_y(&mut self, y: u16) {
        self.current_y = y;
    }
}

/// 布局数据
///
/// 这是一个 no_std 兼容的数据结构，用于存储渲染所需的数据
#[derive(Debug, Clone)]
pub struct LayoutData {
    fields: IndexMap<heapless::String<32>, LayoutValue, BuildHasherDefault<FnvHasher>, 32>,
}

impl Default for LayoutData {
    fn default() -> Self {
        Self::new()
    }
}

impl PartialEq for LayoutData {
    fn eq(&self, other: &Self) -> bool {
        self.fields == other.fields
    }
}

impl LayoutData {
    /// 创建空的布局数据
    pub fn new() -> Self {
        Self {
            fields: IndexMap::new(),
        }
    }

    /// 设置字符串字段
    pub fn set_string(&mut self, key: &str, value: &str) -> Result<(), ()> {
        let key_str = heapless::String::from_str(key).map_err(|_| ())?;
        let value_str = heapless::String::from_str(value).map_err(|_| ())?;
        self.fields
            .insert(key_str, LayoutValue::String(value_str))
            .map_err(|_| ())?;
        Ok(())
    }

    /// 设置整数字段
    pub fn set_i64(&mut self, key: &str, value: i64) -> Result<(), ()> {
        let key_str = heapless::String::from_str(key).map_err(|_| ())?;
        self.fields
            .insert(key_str, LayoutValue::I64(value))
            .map_err(|_| ())?;
        Ok(())
    }

    /// 设置浮点字段
    pub fn set_f64(&mut self, key: &str, value: f64) -> Result<(), ()> {
        let key_str = heapless::String::from_str(key).map_err(|_| ())?;
        self.fields
            .insert(key_str, LayoutValue::F64(value))
            .map_err(|_| ())?;
        Ok(())
    }

    /// 设置布尔字段
    pub fn set_bool(&mut self, key: &str, value: bool) -> Result<(), ()> {
        let key_str = heapless::String::from_str(key).map_err(|_| ())?;
        self.fields
            .insert(key_str, LayoutValue::Bool(value))
            .map_err(|_| ())?;
        Ok(())
    }

    /// 设置数组字段
    pub fn set_array(&mut self, key: &str, value: heapless::Vec<LayoutData, 16>) -> Result<(), ()> {
        let key_str = heapless::String::from_str(key).map_err(|_| ())?;
        self.fields
            .insert(key_str, LayoutValue::Array(Box::new(value)))
            .map_err(|_| ())?;
        Ok(())
    }

    /// 获取字符串值
    pub fn get_string(&self, key: &str) -> Option<heapless::String<64>> {
        let key_str = heapless::String::from_str(key).ok()?;
        match self.fields.get(&key_str)? {
            LayoutValue::String(s) => {
                let mut result = heapless::String::new();
                result.push_str(s).ok()?;
                Some(result)
            }
            LayoutValue::I64(v) => {
                let mut result = heapless::String::new();
                write!(result, "{}", v).ok()?;
                Some(result)
            }
            LayoutValue::F64(v) => {
                let mut result = heapless::String::new();
                write!(result, "{}", v).ok()?;
                Some(result)
            }
            LayoutValue::Bool(v) => {
                let mut result = heapless::String::new();
                write!(result, "{}", v).ok()?;
                Some(result)
            }
            _ => None,
        }
    }

    /// 获取整数值
    pub fn get_i64(&self, key: &str) -> Option<i64> {
        let key_str = heapless::String::from_str(key).ok()?;
        match self.fields.get(&key_str)? {
            LayoutValue::I64(v) => Some(*v),
            _ => None,
        }
    }

    /// 获取浮点值
    pub fn get_f64(&self, key: &str) -> Option<f64> {
        let key_str = heapless::String::from_str(key).ok()?;
        match self.fields.get(&key_str)? {
            LayoutValue::F64(v) => Some(*v),
            _ => None,
        }
    }

    /// 获取布尔值
    pub fn get_bool(&self, key: &str) -> Option<bool> {
        let key_str = heapless::String::from_str(key).ok()?;
        match self.fields.get(&key_str)? {
            LayoutValue::Bool(v) => Some(*v),
            _ => None,
        }
    }

    /// 获取数组值
    pub fn get_array(&self, key: &str) -> Option<&heapless::Vec<LayoutData, 16>> {
        let key_str = heapless::String::from_str(key).ok()?;
        match self.fields.get(&key_str)? {
            LayoutValue::Array(arr) => Some(arr.as_ref()),
            _ => None,
        }
    }

    /// 获取字段
    pub fn get(&self, key: &str) -> Option<&LayoutData> {
        // 简单实现，返回自身（用于条件判断）
        let key_str = heapless::String::from_str(key).ok()?;
        self.fields.get(&key_str)?;
        Some(self)
    }

    /// 获取所有字段名
    pub fn get_fields(&self) -> heapless::Vec<&str, 16> {
        self.fields.keys().map(|s| s.as_str()).collect()
    }
}

/// 布局值类型
#[derive(Debug, Clone)]
pub enum LayoutValue {
    String(heapless::String<64>),
    I64(i64),
    F64(f64),
    Bool(bool),
    Array(Box<heapless::Vec<LayoutData, 16>>),
}

impl PartialEq for LayoutValue {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (LayoutValue::String(a), LayoutValue::String(b)) => a == b,
            (LayoutValue::I64(a), LayoutValue::I64(b)) => a == b,
            (LayoutValue::F64(a), LayoutValue::F64(b)) => a == b,
            (LayoutValue::Bool(a), LayoutValue::Bool(b)) => a == b,
            (LayoutValue::Array(a), LayoutValue::Array(b)) => a.as_ref() == b.as_ref(),
            _ => false,
        }
    }
}

impl LayoutValue {
    /// 转换为字符串
    pub fn to_string(&self) -> heapless::String<64> {
        let mut result = heapless::String::new();
        match self {
            LayoutValue::String(s) => {
                let _ = result.push_str(s);
            }
            LayoutValue::I64(v) => {
                let _ = write!(result, "{}", v);
            }
            LayoutValue::F64(v) => {
                let _ = write!(result, "{}", v);
            }
            LayoutValue::Bool(v) => {
                let _ = write!(result, "{}", v);
            }
            LayoutValue::Array(_) => {
                let _ = result.push_str("[...]");
            }
        }
        result
    }
}

impl PartialOrd for LayoutValue {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        match (self, other) {
            (LayoutValue::I64(a), LayoutValue::I64(b)) => a.partial_cmp(b),
            (LayoutValue::F64(a), LayoutValue::F64(b)) => a.partial_cmp(b),
            (LayoutValue::String(a), LayoutValue::String(b)) => a.partial_cmp(b),
            _ => None,
        }
    }
}

impl PartialEq<LayoutValue> for LayoutData {
    fn eq(&self, other: &LayoutValue) -> bool {
        // 简单实现，比较第一个字段
        if let Some((_, v)) = self.fields.iter().next() {
            return v == other;
        }
        false
    }
}

impl PartialOrd<LayoutValue> for LayoutData {
    fn partial_cmp(&self, other: &LayoutValue) -> Option<core::cmp::Ordering> {
        if let Some((_, v)) = self.fields.iter().next() {
            return v.partial_cmp(other);
        }
        None
    }
}

use core::fmt::Write;
