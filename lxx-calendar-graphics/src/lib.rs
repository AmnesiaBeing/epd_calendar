//! lxx-calendar-graphics — 墨水屏日历图形渲染库
//!
//! 本库提供墨水屏图形渲染功能，支持：
//! - 文本渲染（多字体大小）
//! - 图标渲染（SVG 预渲染为位图）
//! - 布局渲染（inksight 风格的 JSON 布局定义）
//!
//! # 使用示例
//!
//! ```rust,ignore
//! use lxx_calendar_graphics::{Renderer, LayoutData, LayoutParser};
//!
//! let mut renderer = Renderer::<384000>::new(800, 480);
//!
//! // 解析 JSON 布局
//! let layout_json = r#"{
//!     "body": [
//!         {"type": "big_number", "field": "day", "font_size": 72},
//!         {"type": "text", "field": "month_cn", "font_size": 18}
//!     ]
//! }"#;
//!
//! // 解析数据
//! let data = LayoutParser::parse_data(r#"{"day": 13, "month_cn": "三月"}"#)?;
//!
//! // 渲染
//! renderer.render_from_json(layout_json, &data)?;
//! ```

#![no_std]

extern crate alloc;

pub mod assets;
pub mod parser;
pub mod renderer;

pub use parser::{LayoutParser, ParseError};
pub use renderer::{IconRenderer, LayoutData, LayoutEngine, LayoutRenderer, LayoutValue, Renderer, TextRenderer};

// 重新导出常用的类型
pub use lxx_calendar_common::layout;
