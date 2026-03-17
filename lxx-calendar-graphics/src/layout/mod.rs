//! JSON 布局系统
//!
//! 提供通过 JSON 配置定义墨水屏显示内容和布局的能力
//!
//! # 功能特性
//!
//! - **JSON 驱动**: 通过 JSON 配置文件定义显示模式，无需修改代码
//! - **灵活的布局块**: 支持文本、图标、分隔线、间距、区块等多种布局元素
//! - **条件渲染**: 根据数据内容动态显示/隐藏元素
//! - **模板支持**: 使用模板字符串格式化输出
//! - **多种对齐方式**: 支持水平/垂直对齐
//!
//! # 使用示例
//!
//! ```rust,ignore
//! use lxx_calendar_graphics::layout::{ModeLoader, LayoutRenderer, types::ModeDefinition};
//! use alloc::collections::BTreeMap;
//!
//! // 1. 创建模式加载器
//! let mut loader = ModeLoader::new();
//!
//! // 2. 加载模式定义
//! let mode_json = r#"{
//!     "mode_id": "POETRY",
//!     "display_name": "每日诗词",
//!     "layout": {
//!         "status_bar": { "show_date": true, "show_weather": true },
//!         "body": {
//!             "blocks": [
//!                 {
//!                     "type": "section",
//!                     "title": "📖 今日诗词",
//!                     "children": [
//!                         { "type": "text", "field": "poetry_title", "font_size": 18 },
//!                         { "type": "text", "field": "poetry_content", "font_size": 14 }
//!                     ]
//!                 }
//!             ]
//!         },
//!         "footer": { "label": "POETRY" }
//!     }
//! }"#;
//! loader.load_from_json(mode_json).unwrap();
//!
//! // 3. 准备数据
//! let mut data = BTreeMap::new();
//! data.insert("poetry_title".to_string(), "静夜思".to_string());
//! data.insert("poetry_content".to_string(), "床前明月光，疑是地上霜".to_string());
//!
//! // 4. 渲染
//! let mode = loader.get_mode("POETRY").unwrap();
//! let renderer = LayoutRenderer::new();
//! renderer.render(&mut framebuffer, &mode.layout, &data, "POETRY").unwrap();
//! ```
//!
//! # 布局块类型
//!
//! - `text`: 文本块
//! - `icon`: 图标块
//! - `separator`: 分隔线
//! - `spacer`: 间距
//! - `section`: 区块（带标题）
//! - `vstack`: 垂直堆叠
//! - `conditional`: 条件渲染
//! - `big_number`: 大号数字
//! - `progress_bar`: 进度条
//!
//! # 数据字段
//!
//! 渲染时需要提供数据上下文，常用的字段包括：
//! - `date_str`: 日期字符串
//! - `weather_str`: 天气描述
//! - `battery_pct`: 电池百分比
//! - `poetry_title`: 诗词标题
//! - `poetry_content`: 诗词内容
//! - 等等...

extern crate alloc;

pub mod types;
pub mod parser;
pub mod renderer;

// 重新导出常用类型
pub use types::{
    BodyConfig, Condition, ContentConfig, FooterConfig, LayoutBlock, LayoutDefinition,
    LocalSource, ModeDefinition, RenderContext, StatusBarConfig, TextAlign, VerticalAlign,
    LineStyle,
};

pub use parser::ModeLoader;
pub use renderer::LayoutRenderer;
