//! Render module for the EPD Calendar application
//! This module provides all rendering functionality including layout processing, text, image, and graphics rendering

pub mod layout;
pub mod text;
pub mod image;
pub mod graphics;

// Re-export key types and functions for easier access
pub use layout::engine::LayoutEngine;
pub use layout::nodes::LayoutNode;
pub use layout::context::LayoutContext;
pub use layout::loader::LayoutLoader;
pub use layout::evaluator::LayoutEvaluator;
