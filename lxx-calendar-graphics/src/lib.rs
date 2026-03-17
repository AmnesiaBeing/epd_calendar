//! lxx-calendar-graphics — 墨水屏日历图形渲染库
//!
//! 本库提供墨水屏图形渲染功能，支持：
//! - 文本渲染（多字体大小）
//! - 图标渲染（SVG 预渲染为位图）
//! - 布局渲染（JSON 布局定义）
//!
//! # 使用示例
//!
//! ```rust,ignore
//! use lxx_calendar_graphics::{Renderer, LayoutRenderer, ModeLoader};
//! use alloc::collections::BTreeMap;
//!
//! let mut renderer = Renderer::<384000>::new(800, 480);
//!
//! // 1. 创建模式加载器并加载 JSON 布局
//! let mut loader = ModeLoader::new();
//! let mode_json = r#"{
//!     "mode_id": "CALENDAR",
//!     "display_name": "日历",
//!     "layout": {
//!         "status_bar": { "show_date": true, "show_weather": true },
//!         "body": {
//!             "blocks": [
//!                 {"type": "big_number", "field": "day", "font_size": 72},
//!                 {"type": "text", "field": "month_cn", "font_size": 18}
//!             ]
//!         },
//!         "footer": { "label": "CALENDAR" }
//!     }
//! }"#;
//! loader.load_from_json(mode_json).unwrap();
//!
//! // 2. 准备数据
//! let mut data = BTreeMap::new();
//! data.insert("day".to_string(), "13".to_string());
//! data.insert("month_cn".to_string(), "三月".to_string());
//!
//! // 3. 渲染
//! let mode = loader.get_mode("CALENDAR").unwrap();
//! let layout_renderer = LayoutRenderer::new();
//! layout_renderer.render(&mut renderer.framebuffer, &mode.layout, &data, "CALENDAR").unwrap();
//! ```

#![no_std]

extern crate alloc;

pub mod assets;
pub mod layout;
pub mod renderer;

// 重新导出常用类型
pub use layout::{
    BodyConfig, Condition, ContentConfig, FooterConfig, LayoutBlock, LayoutDefinition,
    LocalSource, ModeDefinition, ModeLoader, RenderContext, StatusBarConfig, TextAlign,
    VerticalAlign, LineStyle,
};
pub use renderer::{Color, Framebuffer, IconRenderer, Renderer, TextRenderer};

// 重新导出布局渲染器
pub use layout::renderer::LayoutRenderer;
