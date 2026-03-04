#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub mod assets;
pub mod framebuffer;
pub mod renderer;

// 重新导出常用类型
pub use framebuffer::{Color, Framebuffer, FramebufferError};
pub use renderer::{Renderer, TextRenderer, IconRenderer, LayoutRenderer};
