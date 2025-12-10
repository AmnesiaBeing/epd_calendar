//! Rendering context for maintaining rendering state

use crate::kernel::render::layout::{nodes::LayoutNode, evaluator::ConditionEvaluator};
use crate::kernel::render::{text, image, graphics};
use crate::common::error::{Result, AppError};

/// Render context containing rendering state and utilities
pub struct RenderContext {
    /// Condition evaluator for evaluating visibility conditions
    evaluator: ConditionEvaluator,
    /// Current drawing offset
    offset: (i32, i32),
    /// Current dimensions
    dimensions: (u32, u32),
    /// Rendering state
    state: RenderState,
}

/// Rendering state
pub struct RenderState {
    /// Current background color
    current_background: Option<String>,
    /// Current text color
    current_text_color: Option<String>,
    /// Current font family
    current_font_family: Option<String>,
    /// Current font size
    current_font_size: Option<u32>,
}

impl Default for RenderState {
    fn default() -> Self {
        Self {
            current_background: None,
            current_text_color: None,
            current_font_family: None,
            current_font_size: None,
        }
    }
}

impl RenderContext {
    /// Create a new RenderContext instance
    pub fn new() -> Result<Self> {
        Ok(Self {
            evaluator: ConditionEvaluator::new()?,
            offset: (0, 0),
            dimensions: (800, 480), // Default dimensions
            state: RenderState::default(),
        })
    }

    /// Evaluate visibility condition for a node
    pub fn evaluate_visibility(&mut self, node: &LayoutNode) -> Result<bool> {
        if let Some(condition) = &node.style.visible {
            self.evaluator.evaluate(condition)
        } else {
            Ok(true) // Default to visible if no condition
        }
    }

    /// Render a single node
    pub fn render_node(&mut self, node: &LayoutNode) -> Result<()> {
        // Save current state
        let saved_state = self.state.clone();

        // Update state with node style
        self.update_state(&node.style);

        // Calculate absolute position
        let abs_x = self.calculate_absolute_position(&node.geometry.x, true)?;
        let abs_y = self.calculate_absolute_position(&node.geometry.y, false)?;
        let width = self.calculate_dimension(&node.geometry.width, true)?;
        let height = self.calculate_dimension(&node.geometry.height, false)?;

        // Render based on node type
        match &node.node_type {
            nodes::NodeType::Container => {
                // Container nodes just update context for children
                self.offset = (abs_x as i32, abs_y as i32);
            }
            nodes::NodeType::Text => {
                if let Some(content) = &node.content {
                    text::render_text(content, abs_x, abs_y, width, height, &self.state)?;
                }
            }
            nodes::NodeType::Image => {
                if let Some(image_path) = &node.content {
                    image::render_image(image_path, abs_x, abs_y, width, height)?;
                }
            }
            nodes::NodeType::Rectangle => {
                graphics::render_rectangle(abs_x, abs_y, width, height, &self.state)?;
            }
            nodes::NodeType::Circle => {
                graphics::render_circle(abs_x, abs_y, width, height, &self.state)?;
            }
            nodes::NodeType::Line => {
                graphics::render_line(abs_x, abs_y, width, height, &self.state)?;
            }
        }

        // Restore saved state
        self.state = saved_state;

        Ok(())
    }

    // Style-related methods will be added as needed
    // Currently, we're using individual style properties in each node type

    /// Calculate absolute position from relative/absolute value
    fn calculate_absolute_position(&self, value: &str, is_x: bool) -> Result<u32> {
        if value.ends_with('%') {
            // Percentage based position
            let percentage = value.trim_end_matches('%').parse::<f32>()? / 100.0;
            let dimension = if is_x {
                self.dimensions.0 as f32
            } else {
                self.dimensions.1 as f32
            };
            Ok((dimension * percentage) as u32 + self.offset.0 as u32)
        } else {
            // Absolute pixel position
            Ok(value.parse::<u32>()? + self.offset.0 as u32)
        }
    }

    /// Calculate dimension from relative/absolute value
    fn calculate_dimension(&self, value: &str, is_width: bool) -> Result<u32> {
        if value.ends_with('%') {
            // Percentage based dimension
            let percentage = value.trim_end_matches('%').parse::<f32>()? / 100.0;
            let dimension = if is_width {
                self.dimensions.0 as f32
            } else {
                self.dimensions.1 as f32
            };
            Ok((dimension * percentage) as u32)
        } else {
            // Absolute pixel dimension
            Ok(value.parse::<u32>()?)
        }
    }
}
