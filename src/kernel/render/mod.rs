//! 渲染引擎模块
//! 整合文本/图标渲染，调用布局规则完成800x480信息面板的整体渲染

pub mod icon_renderer;
pub mod layout;
pub mod text_renderer;

use core::fmt::Debug;

use embedded_graphics::draw_target::DrawTarget;
use epd_waveshare::color::QuadColor;

use self::icon_renderer::IconRenderer;
use self::layout::{get_global_layout_rules, get_layout_element};
use self::text_renderer::{TextAlign, TextRenderConfig, TextRenderer};
use crate::assets::generated_fonts::FontSize;
use crate::common::error::{AppError, Result};
use crate::kernel::data::scheduler::DataSourceRegistry;
use crate::kernel::data::types::CacheKeyValueMap;

/// 全局渲染引擎实例
pub static DEFAULT_ENGINE: RenderEngine = RenderEngine;

/// 渲染引擎
pub struct RenderEngine;

impl RenderEngine {
    /// 渲染整个布局到显示缓冲区
    pub fn render_layout<DT>(
        &self,
        target: &mut DT,
        data_source_registry: &DataSourceRegistry,
        cache: &CacheKeyValueMap,
    ) -> Result<bool>
    where
        DT: DrawTarget<Color = QuadColor>,
        DT::Error: Debug,
    {
        log::info!("开始渲染布局");
        let layout_rules = get_global_layout_rules();

        // 1. 渲染所有布局元素
        for (id, element) in &layout_rules.elements {
            match element.element_type {
                layout::LayoutElementType::Text => {
                    self.render_text_element(target, element, cache)?;
                }
                layout::LayoutElementType::Icon => {
                    self.render_icon_element(target, element, cache)?;
                }
                layout::LayoutElementType::Line => {
                    self.render_line_element(target, element)?;
                }
                layout::LayoutElementType::Container => {
                    // 容器元素无需渲染
                    continue;
                }
            }
        }

        Ok(true)
    }

    /// 渲染文本元素
    fn render_text_element<DT>(
        &self,
        target: &mut DT,
        element: &layout::LayoutElement,
        cache: &CacheKeyValueMap,
    ) -> Result<()>
    where
        DT: DrawTarget<Color = QuadColor>,
        DT::Error: Debug,
    {
        // 1. 获取数据
        let text = if let Some(data_key) = element.data_key {
            self.get_cache_string(cache, data_key)?
        } else {
            return Ok(());
        };

        // 2. 获取样式配置
        let font_size = match element.style.font_size {
            Some(16) => FontSize::Small,
            Some(24) => FontSize::Medium,
            Some(40) => FontSize::Large,
            _ => FontSize::Medium,
        };

        let align = match element.style.text_align {
            Some(a) => TextAlign::from(&a),
            None => TextAlign::Left,
        };

        // 3. 特殊处理格言（自动对齐）
        let final_align = if element.id == "motto_content" {
            TextRenderer::auto_align(&text, font_size, element.style.width)?
        } else {
            align
        };

        // 4. 构建渲染配置
        let config = TextRenderConfig {
            font_size,
            align: final_align,
            max_width: Some(element.style.width),
            max_lines: Some(3),
        };

        // 5. 渲染文本
        TextRenderer::render_text(target, &text, element.style.x, element.style.y, config)?;

        Ok(())
    }

    /// 渲染图标元素
    fn render_icon_element<DT>(
        &self,
        target: &mut DT,
        element: &layout::LayoutElement,
        cache: &CacheKeyValueMap,
    ) -> Result<()>
    where
        DT: DrawTarget<Color = QuadColor>,
        DT::Error: Debug,
    {
        // 1. 获取图标ID
        let icon_id = if let Some(data_key) = element.data_key {
            let value = self.get_cache_string(cache, data_key)?;

            // 替换图标ID模式
            if let Some(pattern) = element.style.icon_id_pattern {
                pattern.replace("{}", &value)
            } else {
                value
            }
        } else {
            return Ok(());
        };

        // 2. 渲染图标
        IconRenderer::render_icon(target, &icon_id, element.style.x, element.style.y)?;

        Ok(())
    }

    /// 渲染线条元素
    fn render_line_element<DT>(
        &self,
        target: &mut DT,
        element: &layout::LayoutElement,
    ) -> Result<()>
    where
        DT: DrawTarget<Color = QuadColor>,
        DT::Error: Debug,
    {
        // 计算线条的起始/结束坐标
        let (x1, y1, x2, y2) = if element.style.width > element.style.height {
            // 水平线
            (
                element.style.x,
                element.style.y,
                element.style.x + element.style.width,
                element.style.y,
            )
        } else {
            // 垂直线
            (
                element.style.x,
                element.style.y,
                element.style.x,
                element.style.y + element.style.height,
            )
        };

        // 渲染线条
        IconRenderer::render_line(target, x1, y1, x2, y2)?;

        Ok(())
    }

    /// 从缓存获取字符串值
    fn get_cache_string(&self, cache: &CacheKeyValueMap, key: &str) -> Result<String> {
        let value = cache
            .get(key)
            .ok_or_else(|| AppError::CacheMiss(key.to_string()))?;

        match value {
            crate::kernel::data::types::DynamicValue::String(s) => Ok(s.to_string()),
            crate::kernel::data::types::DynamicValue::Integer(i) => Ok(i.to_string()),
            crate::kernel::data::types::DynamicValue::Float(f) => Ok(f.to_string()),
            crate::kernel::data::types::DynamicValue::Boolean(b) => Ok(b.to_string()),
        }
    }
}
