//! JSON 解析器模块
//! 解析 inksight 风格的布局 JSON 定义

pub mod layout_parser;

pub use layout_parser::{LayoutParser, ParseError};
