// src/render/mod.rs
mod components;

mod render_engine;
pub use render_engine::RenderEngine;

mod image_renderer;
pub use image_renderer::draw_binary_image;

mod text_renderer;
pub use text_renderer::TextRenderer;
