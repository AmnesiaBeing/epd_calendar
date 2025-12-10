//! 渲染模块
//! 负责将布局和数据渲染到屏幕上

pub mod graphics;
pub mod image;
pub mod layout;
pub mod text;

pub use graphics::{GraphicsRenderError, GraphicsRenderer};
pub use image::{ImageRenderError, ImageRenderer};
pub use layout::*;
pub use text::TextRenderer;
