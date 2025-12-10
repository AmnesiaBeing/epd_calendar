//! Layout evaluator for processing conditions and placeholders

use crate::common::error::{Result, AppError};
use crate::kernel::data::app_data::{AppData, AppDataType};
use crate::kernel::render::layout::nodes::{LayoutNode, Container, Text, Icon, Line, Rectangle, Circle};
use crate::kernel::render::layout::context::LayoutContext;
use heapless::{String, Vec};

/// Layout evaluator for evaluating conditions and processing placeholders
pub struct LayoutEvaluator {
    // No fields needed - we use LayoutContext for data access
}

impl LayoutEvaluator {
    /// Create a new LayoutEvaluator instance
    pub fn new() -> Self {
        Self {}
    }

    /// Evaluate the entire layout recursively
    pub fn evaluate_layout(&self, node: &mut LayoutNode, context: &LayoutContext) -> Result<()> {
        // Evaluate visibility condition
        if !self.evaluate_node_visibility(node, context)? {
            // If node is not visible, we don't need to process it further
            return Ok(());
        }

        // Process node content and attributes
        self.process_node_content(node, context)?;

        // Process children recursively
        match node {
            LayoutNode::Container(container) => {
                for child in &mut container.children {
                    self.evaluate_layout(&mut *child.node, context)?;
                }
            },
            _ => {},
        }

        Ok(())
    }

    /// Evaluate node visibility based on condition
    fn evaluate_node_visibility(&self, node: &mut LayoutNode, context: &LayoutContext) -> Result<bool> {
        let condition = match node {
            LayoutNode::Container(container) => container.condition.as_deref(),
            LayoutNode::Text(text) => None,
            LayoutNode::Icon(icon) => None,
            LayoutNode::Line(line) => None,
            LayoutNode::Rectangle(rect) => None,
            LayoutNode::Circle(circle) => None,
        };

        if let Some(condition) = condition {
            self.evaluate_condition(condition, context)
        } else {
            Ok(true)
        }
    }

    /// Process node content and replace placeholders
    fn process_node_content(&self, node: &mut LayoutNode, context: &LayoutContext) -> Result<()> {
        match node {
            LayoutNode::Text(text) => {
                let processed_content = self.process_placeholder(&text.content, context)?;
                text.content = processed_content;
            },
            LayoutNode::Icon(icon) => {
                let processed_icon_id = self.process_placeholder(&icon.icon_id, context)?;
                icon.icon_id = processed_icon_id;
            },
            _ => {},
        }

        Ok(())
    }

    /// Evaluate a condition string
    fn evaluate_condition(&self, condition: &str, context: &LayoutContext) -> Result<bool> {
        // Simple condition evaluation (can be extended with more complex logic)
        // Currently supports basic placeholder checks: ${placeholder} == "value"
        if condition.contains("==") {
            let parts: Vec<&str> = condition.split("==").map(|s| s.trim()).collect();
            if parts.len() != 2 {
                return Err(AppError::LayoutConditionParse);
            }

            let left = self.process_placeholder_str(parts[0], context)?;
            let right = self.process_placeholder_str(parts[1], context)?.trim_matches('"');

            Ok(left == right)
        } else if condition.starts_with("!") {
            // Negation
            let inner = &condition[1..].trim();
            Ok(!self.evaluate_condition(inner, context)?)
        } else {
            // Simple boolean check
            Ok(condition.parse::<bool>()?)
        }
    }

    /// Process a string and replace placeholders
    fn process_placeholder(&self, input: &heapless::String<128>, context: &LayoutContext) -> Result<heapless::String<128>> {
        let input_str = input.as_str();
        let processed_str = self.process_placeholder_str(input_str, context)?;
        
        // Convert to heapless::String
        let mut result = heapless::String::<128>::new();
        result.push_str(processed_str).map_err(|_| AppError::LayoutPlaceholderNotFound)?;
        
        Ok(result)
    }

    /// Process a string slice and replace placeholders
    fn process_placeholder_str(&self, input: &str, context: &LayoutContext) -> Result<&str> {
        // Check if it's a placeholder: ${placeholder}
        if input.starts_with("$") && input.contains("{") && input.ends_with("}") {
            let placeholder_name = input.trim_start_matches("$").trim_start_matches("{").trim_end_matches("}");
            context.get_placeholder(placeholder_name).ok_or(AppError::LayoutPlaceholderNotFound)
        } else {
            Ok(input)
        }
    }
}
