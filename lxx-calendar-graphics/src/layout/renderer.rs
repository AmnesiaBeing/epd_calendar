//! JSON 布局渲染引擎
//!
//! 根据 JSON 布局定义将数据渲染到帧缓冲区

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use heapless::Vec;

use super::types::*;
use crate::renderer::{Color, Framebuffer, TextRenderer};
use lxx_calendar_common::SystemResult;

/// 布局渲染器 - 根据 JSON 布局定义渲染内容
pub struct LayoutRenderer {
    text_renderer: TextRenderer,
}

impl LayoutRenderer {
    /// 创建新的布局渲染器
    pub fn new() -> Self {
        Self {
            text_renderer: TextRenderer::new(),
        }
    }

    /// 渲染完整的布局定义
    pub fn render<const SIZE: usize>(
        &self,
        framebuffer: &mut Framebuffer<SIZE>,
        layout: &LayoutDefinition,
        data: &BTreeMap<String, String>,
        mode_id: &str,
    ) -> SystemResult<()> {
        let screen_width = framebuffer.width() as u32;
        let screen_height = framebuffer.height() as u32;

        let mut ctx = RenderContext::new(screen_width, screen_height, data);

        // 1. 渲染状态栏
        if let Some(status_bar) = &layout.status_bar {
            self.render_status_bar(framebuffer, &mut ctx, status_bar)?;
        }

        // 2. 渲染主体内容
        self.render_body(framebuffer, &mut ctx, &layout.body)?;

        // 3. 渲染页脚
        if let Some(footer) = &layout.footer {
            self.render_footer(framebuffer, &mut ctx, footer, mode_id)?;
        }

        Ok(())
    }

    /// 渲染状态栏
    fn render_status_bar<const SIZE: usize>(
        &self,
        framebuffer: &mut Framebuffer<SIZE>,
        ctx: &mut RenderContext,
        config: &StatusBarConfig,
    ) -> SystemResult<()> {
        let y = 10u16;
        let mut x = 10u16;

        // 渲染日期
        if config.show_date {
            if let Some(date_str) = ctx.data.get("date_str") {
                self.text_renderer.render(framebuffer, x, y, date_str)?;
                x += 150;
            }
        }

        // 渲染天气
        if config.show_weather {
            if let Some(weather_str) = ctx.data.get("weather_str") {
                self.text_renderer.render(framebuffer, x, y, weather_str)?;
            }
        }

        // 渲染电池
        if config.show_battery {
            if let Some(battery_pct) = ctx.data.get("battery_pct") {
                let bat_x = (ctx.screen_width - 60) as u16;
                self.text_renderer.render(framebuffer, bat_x, y, battery_pct)?;
            }
        }

        // 绘制分隔线
        let line_y = ctx.status_bar_height as u16;
        let line_width = config.line_width.unwrap_or(1);

        if config.dashed {
            self.draw_dashed_line(framebuffer, 0, line_y, ctx.screen_width as u16, line_width)?;
        } else {
            let _ = framebuffer.draw_horizontal_line(0, line_y, ctx.screen_width as u16, Color::Black);
        }

        Ok(())
    }

    /// 渲染主体内容
    fn render_body<const SIZE: usize>(
        &self,
        framebuffer: &mut Framebuffer<SIZE>,
        ctx: &mut RenderContext,
        body: &BodyConfig,
    ) -> SystemResult<()> {
        // 计算垂直居中对齐（如果需要）
        if body.vertical_align == Some(VerticalAlign::Center) {
            if body.blocks.len() == 1 {
                // 单个块垂直居中
                if let Some(block) = body.blocks.first() {
                    let content_height = self.measure_block_height(block, ctx);
                    let available_height = ctx.screen_height - ctx.status_bar_height - ctx.footer_height;
                    let offset = (available_height.saturating_sub(content_height)) / 2;
                    ctx.current_y = ctx.status_bar_height + offset;
                }
            }
        }

        // 渲染所有块
        for block in body.blocks.iter() {
            if ctx.remaining_height() < 10 {
                break;
            }
            self.render_block(framebuffer, ctx, block)?;
        }

        Ok(())
    }

    /// 渲染单个布局块
    fn render_block<const SIZE: usize>(
        &self,
        framebuffer: &mut Framebuffer<SIZE>,
        ctx: &mut RenderContext,
        block: &LayoutBlock,
    ) -> SystemResult<()> {
        match block {
            LayoutBlock::Text {
                field,
                font_size,
                align,
                max_lines,
                margin_x,
                template,
            } => self.render_text(
                framebuffer,
                ctx,
                field,
                *font_size,
                align,
                *max_lines,
                *margin_x,
                template.as_deref(),
            ),

            LayoutBlock::Icon { name, size } => {
                self.render_icon(framebuffer, ctx, name, *size)
            }

            LayoutBlock::Separator {
                style,
                line_width,
                margin_x,
                width,
            } => self.render_separator(
                framebuffer,
                ctx,
                style,
                *line_width,
                *margin_x,
                *width,
            ),

            LayoutBlock::Spacer { height } => {
                ctx.current_y += *height as u32;
                Ok(())
            }

            LayoutBlock::Section {
                title,
                icon,
                children,
            } => self.render_section(framebuffer, ctx, title, icon.as_deref(), children),

            LayoutBlock::VStack { spacing, children } => {
                for child in children.iter() {
                    if ctx.remaining_height() < 10 {
                        break;
                    }
                    self.render_block(framebuffer, ctx, child)?;
                    ctx.current_y += *spacing as u32;
                }
                Ok(())
            }

            LayoutBlock::Conditional {
                field,
                condition,
                then_children,
                else_children,
            } => {
                let should_render = self.evaluate_condition(ctx, field, condition);
                let children = if should_render {
                    then_children
                } else {
                    else_children.as_deref().unwrap_or(&[])
                };

                for child in children.iter() {
                    if ctx.remaining_height() < 10 {
                        break;
                    }
                    self.render_block(framebuffer, ctx, child)?;
                }
                Ok(())
            }

            LayoutBlock::BigNumber {
                field,
                font_size,
                align,
                unit,
            } => self.render_big_number(
                framebuffer,
                ctx,
                field,
                *font_size,
                align,
                unit.as_deref(),
            ),

            LayoutBlock::ProgressBar {
                field,
                max_field,
                width,
                height,
                margin_x,
            } => self.render_progress_bar(
                framebuffer,
                ctx,
                field,
                max_field,
                *width,
                *height,
                *margin_x,
            ),
        }
    }

    /// 渲染文本块
    fn render_text<const SIZE: usize>(
        &self,
        framebuffer: &mut Framebuffer<SIZE>,
        ctx: &mut RenderContext,
        field: &str,
        font_size: u16,
        align: &TextAlign,
        max_lines: Option<u16>,
        margin_x: Option<i16>,
        template: Option<&str>,
    ) -> SystemResult<()> {
        // 获取文本内容
        let text = match ctx.get_field(field) {
            Some(t) => t.as_str(),
            None => return Ok(()), // 字段不存在，跳过
        };

        // 应用模板（如果有）
        let final_text = match template {
            Some(tmpl) => self.resolve_template(tmpl, ctx.data),
            None => String::from(text),
        };

        let margin = margin_x.map(|m| m as u32).unwrap_or(ctx.default_margin_x());
        let max_width = ctx.available_width;

        // 文本换行
        let lines = self.wrap_text(&final_text, font_size, max_width);

        // 限制行数并渲染
        let max_lines_val = max_lines.unwrap_or(lines.len() as u16) as usize;
        let lines_to_render: Vec<_, 16> = lines.iter().take(max_lines_val).map(|s| s.as_str()).collect();

        for line in lines_to_render.iter() {
            if ctx.remaining_height() < font_size as u32 {
                break;
            }

            let line_to_draw = *line;
            let line_width = self.measure_text_width(line_to_draw, font_size);
            let x = match align {
                TextAlign::Left => margin as u16,
                TextAlign::Center => ((ctx.screen_width - line_width) / 2) as u16,
                TextAlign::Right => (ctx.screen_width - margin - line_width as u32) as u16,
            };

            self.text_renderer
                .render_with_size(framebuffer, x, ctx.current_y as u16, line_to_draw, font_size)?;

            ctx.current_y += font_size as u32 + 4;
        }

        Ok(())
    }

    /// 渲染图标
    fn render_icon<const SIZE: usize>(
        &self,
        framebuffer: &mut Framebuffer<SIZE>,
        ctx: &mut RenderContext,
        _name: &str,
        size: u16,
    ) -> SystemResult<()> {
        // 简化实现：绘制一个矩形占位符
        let x = ctx.default_margin_x() as u16;
        let y = ctx.current_y as u16;
        let _ = framebuffer.draw_rectangle(x, y, size, size, Color::Black);
        ctx.current_y += size as u32 + 4;
        Ok(())
    }

    /// 渲染分隔线
    fn render_separator<const SIZE: usize>(
        &self,
        framebuffer: &mut Framebuffer<SIZE>,
        ctx: &mut RenderContext,
        style: &LineStyle,
        line_width: Option<u16>,
        margin_x: Option<i16>,
        width: Option<u16>,
    ) -> SystemResult<()> {
        let width_val = line_width.unwrap_or(1);
        let margin = margin_x.map(|m| m as u32).unwrap_or(ctx.default_margin_x());
        let y = ctx.current_y as u16;

        let (x1, x2) = match style {
            LineStyle::Short => {
                let line_len = width.unwrap_or(ctx.screen_width as u16 / 4);
                let center = ctx.screen_width as u16 / 2;
                (center - line_len / 2, center + line_len / 2)
            }
            _ => (margin as u16, (ctx.screen_width - margin) as u16),
        };

        match style {
            LineStyle::Solid | LineStyle::Short => {
                let _ = framebuffer.draw_horizontal_line(x1, y, x2 - x1, Color::Black);
            }
            LineStyle::Dashed => {
                self.draw_dashed_line(framebuffer, x1, y, x2 - x1, width_val)?;
            }
            LineStyle::Dotted => {
                self.draw_dotted_line(framebuffer, x1, y, x2 - x1, width_val)?;
            }
        }

        ctx.current_y += width_val as u32 + 4;
        Ok(())
    }

    /// 渲染区块（带标题）
    fn render_section<const SIZE: usize>(
        &self,
        framebuffer: &mut Framebuffer<SIZE>,
        ctx: &mut RenderContext,
        title: &str,
        icon: Option<&str>,
        children: &[LayoutBlock],
    ) -> SystemResult<()> {
        let title_font_size = 14u16;
        let mut x = ctx.default_margin_x() as u16;
        let y = ctx.current_y as u16;

        // 渲染图标（如果有）
        if let Some(_icon_name) = icon {
            // TODO: 渲染实际图标
            let icon_size = 12u16;
            let _ = framebuffer.draw_rectangle(x, y, icon_size, icon_size, Color::Black);
            x += icon_size + 4;
        }

        // 渲染标题
        self.text_renderer
            .render_with_size(framebuffer, x, y, title, title_font_size)?;

        ctx.current_y += title_font_size as u32 + 6;

        // 渲染子块
        for child in children.iter() {
            if ctx.remaining_height() < 10 {
                break;
            }
            self.render_block(framebuffer, ctx, child)?;
        }

        Ok(())
    }

    /// 渲染大号数字
    fn render_big_number<const SIZE: usize>(
        &self,
        framebuffer: &mut Framebuffer<SIZE>,
        ctx: &mut RenderContext,
        field: &str,
        font_size: u16,
        align: &TextAlign,
        unit: Option<&str>,
    ) -> SystemResult<()> {
        let text = match ctx.get_field(field) {
            Some(t) => t.as_str(),
            None => return Ok(()),
        };

        let mut display_text = String::from(text);
        if let Some(u) = unit {
            display_text.push_str(u);
        }

        let line_width = self.measure_text_width(&display_text, font_size);
        let margin = ctx.default_margin_x();
        let x = match align {
            TextAlign::Left => margin as u16,
            TextAlign::Center => ((ctx.screen_width - line_width) / 2) as u16,
            TextAlign::Right => (ctx.screen_width - margin - line_width as u32) as u16,
        };

        self.text_renderer
            .render_with_size(framebuffer, x, ctx.current_y as u16, &display_text, font_size)?;

        ctx.current_y += font_size as u32 + 6;
        Ok(())
    }

    /// 渲染进度条
    fn render_progress_bar<const SIZE: usize>(
        &self,
        framebuffer: &mut Framebuffer<SIZE>,
        ctx: &mut RenderContext,
        field: &str,
        max_field: &str,
        width: u16,
        height: u16,
        margin_x: Option<i16>,
    ) -> SystemResult<()> {
        let value: i32 = ctx
            .get_field(field)
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);
        let max_value: i32 = ctx
            .get_field(max_field)
            .and_then(|v| v.parse().ok())
            .unwrap_or(1);

        let ratio = if max_value > 0 {
            (value as u32 * width as u32 / max_value as u32).min(width as u32)
        } else {
            0
        };

        let margin = margin_x.map(|m| m as u32).unwrap_or(ctx.default_margin_x());
        let x = margin as u16;
        let y = ctx.current_y as u16;

        // 绘制外框
        let _ = framebuffer.draw_rectangle(x, y, width, height, Color::Black);

        // 绘制填充部分
        if ratio > 0 {
            let fill_width = (ratio - 1) as u16;
            let _ = framebuffer.fill_rectangle(x + 1, y + 1, fill_width, height - 2, Color::Black);
        }

        ctx.current_y += height as u32 + 6;
        Ok(())
    }

    /// 评估条件
    fn evaluate_condition(
        &self,
        ctx: &RenderContext,
        field: &str,
        condition: &Condition,
    ) -> bool {
        let value = ctx.get_field(field).map(|s| s.as_str()).unwrap_or("");

        match condition {
            Condition::Exists => !value.is_empty(),
            Condition::Eq { value: expected } => value == expected,
            Condition::NotEq { value: expected } => value != expected,
            Condition::Gt { value: threshold } => {
                value.parse::<i32>().map(|v| v > *threshold).unwrap_or(false)
            }
            Condition::Lt { value: threshold } => {
                value.parse::<i32>().map(|v| v < *threshold).unwrap_or(false)
            }
            Condition::Gte { value: threshold } => {
                value.parse::<i32>().map(|v| v >= *threshold).unwrap_or(false)
            }
            Condition::Lte { value: threshold } => {
                value.parse::<i32>().map(|v| v <= *threshold).unwrap_or(false)
            }
        }
    }

    /// 渲染页脚
    fn render_footer<const SIZE: usize>(
        &self,
        framebuffer: &mut Framebuffer<SIZE>,
        ctx: &mut RenderContext,
        config: &FooterConfig,
        mode_id: &str,
    ) -> SystemResult<()> {
        let footer_height = config.height.unwrap_or(ctx.footer_height as u16);
        let footer_top = (ctx.screen_height - footer_height as u32) as u16;

        // 绘制分隔线
        let line_width = config.line_width.unwrap_or(1);
        if config.dashed {
            self.draw_dashed_line(
                framebuffer,
                0,
                footer_top,
                ctx.screen_width as u16,
                line_width,
            )?;
        } else {
            let _ = framebuffer.draw_horizontal_line(0, footer_top, ctx.screen_width as u16, Color::Black);
        }

        // 渲染标签
        let label = if config.show_mode_name && !config.label.is_empty() {
            config.label.as_str()
        } else if config.show_mode_name {
            mode_id
        } else {
            ""
        };

        if !label.is_empty() {
            let font_size = 10u16;
            let label_width = self.measure_text_width(label, font_size);
            let x = ((ctx.screen_width - label_width) / 2) as u16;
            let y = footer_top + 4;

            self.text_renderer
                .render_with_size(framebuffer, x, y, label, font_size)?;
        }

        Ok(())
    }

    /// 测量块高度（用于垂直居中计算）
    fn measure_block_height(&self, block: &LayoutBlock, ctx: &RenderContext) -> u32 {
        match block {
            LayoutBlock::Text {
                field,
                font_size,
                max_lines,
                ..
            } => {
                let lines = if let Some(text) = ctx.get_field(field) {
                    self.wrap_text(text, *font_size, ctx.available_width)
                } else {
                    Vec::new()
                };
                let line_count = max_lines.map(|m| m.min(lines.len() as u16)).unwrap_or(lines.len() as u16);
                (line_count as u32) * (*font_size as u32 + 4)
            }
            LayoutBlock::Icon { size, .. } => *size as u32 + 4,
            LayoutBlock::Separator { .. } => 8,
            LayoutBlock::Spacer { height } => *height as u32,
            LayoutBlock::Section { children, .. } | LayoutBlock::VStack { children, .. } => {
                children.iter().map(|c| self.measure_block_height(c, ctx)).sum()
            }
            LayoutBlock::Conditional { then_children, .. } => {
                then_children.iter().map(|c| self.measure_block_height(c, ctx)).sum()
            }
            LayoutBlock::BigNumber { font_size, .. } => *font_size as u32 + 6,
            LayoutBlock::ProgressBar { height, .. } => *height as u32 + 6,
        }
    }

    /// 文本换行
    fn wrap_text(&self, text: &str, font_size: u16, max_width: u32) -> Vec<alloc::string::String, 16> {
        let mut lines = Vec::new();
        let mut current_line = alloc::string::String::new();
        let mut current_width = 0u32;

        // 简化的换行逻辑 - 按字符估算宽度
        let char_width = font_size as u32 * 3 / 5; // 估算值

        for ch in text.chars() {
            if ch == '\n' {
                if !current_line.is_empty() {
                    let _ = lines.push(current_line.clone());
                }
                current_line.clear();
                current_width = 0;
                continue;
            }

            let ch_width = if ch.is_ascii() {
                char_width * 2 / 3
            } else {
                char_width // CJK 字符
            };

            if current_width + ch_width > max_width {
                if !current_line.is_empty() {
                    let _ = lines.push(current_line.clone());
                }
                current_line.clear();
                current_line.push(ch);
                current_width = ch_width;
            } else {
                current_line.push(ch);
                current_width += ch_width;
            }
        }

        if !current_line.is_empty() {
            let _ = lines.push(current_line);
        }

        lines
    }

    /// 测量文本宽度
    fn measure_text_width(&self, text: &str, font_size: u16) -> u32 {
        let char_width = font_size as u32 * 3 / 5;
        let mut width = 0u32;

        for ch in text.chars() {
            width += if ch.is_ascii() {
                char_width * 2 / 3
            } else {
                char_width
            };
        }

        width
    }

    /// 绘制虚线
    fn draw_dashed_line<const SIZE: usize>(
        &self,
        framebuffer: &mut Framebuffer<SIZE>,
        x: u16,
        y: u16,
        length: u16,
        _width: u16,
    ) -> SystemResult<()> {
        let dash_len = 4u16;
        let gap_len = 2u16;
        let mut current_x = x;

        while current_x < x + length {
            let dash_end = (current_x + dash_len).min(x + length);
            let _ = framebuffer.draw_horizontal_line(current_x, y, dash_end - current_x, Color::Black);
            current_x = dash_end + gap_len;
        }

        Ok(())
    }

    /// 绘制点线
    fn draw_dotted_line<const SIZE: usize>(
        &self,
        framebuffer: &mut Framebuffer<SIZE>,
        x: u16,
        y: u16,
        length: u16,
        _width: u16,
    ) -> SystemResult<()> {
        let dot_len = 1u16;
        let gap_len = 3u16;
        let mut current_x = x;

        while current_x < x + length {
            let dot_end = (current_x + dot_len).min(x + length);
            let _ = framebuffer.draw_horizontal_line(current_x, y, dot_end - current_x, Color::Black);
            current_x = dot_end + gap_len;
        }

        Ok(())
    }

    /// 解析模板字符串
    fn resolve_template(&self, template: &str, data: &BTreeMap<String, String>) -> String {
        let mut result = String::from(template);
        for (key, value) in data.iter() {
            let placeholder = alloc::format!("{{{}}}", key);
            result = result.replace(&placeholder, value);
        }
        result
    }
}

impl Default for LayoutRenderer {
    fn default() -> Self {
        Self::new()
    }
}
