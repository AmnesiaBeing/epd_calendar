// src/render/mod.rs

pub mod engine;

mod image_renderer;
pub use image_renderer::draw_binary_image;

mod text_renderer;
pub use text_renderer::TextRenderer;

mod graphics_renderer;
pub use graphics_renderer::GraphicsRenderer;
