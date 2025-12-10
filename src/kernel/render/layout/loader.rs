//! Layout loader for loading and deserializing layout definitions

use crate::kernel::render::layout::nodes::LayoutNode;
use crate::common::error::{Result, AppError};
use postcard::from_bytes;

/// Layout loader for loading layout definitions from embedded binary data
pub struct LayoutLoader {
    // No fields needed - we load from embedded data
}

impl LayoutLoader {
    /// Create a new LayoutLoader instance
    pub fn new() -> Self {
        Self {}
    }

    /// Load layout from embedded binary data
    pub fn load_layout(&self) -> Result<LayoutNode> {
        // Import the embedded layout binary data
        // This data is generated at compile time by the builder
        extern "C" {
            static __LAYOUT_BIN_START: u8;
            static __LAYOUT_BIN_END: u8;
        }

        // Get the layout binary data as a slice
        let layout_data = unsafe {
            let start_ptr = &__LAYOUT_BIN_START as *const u8;
            let end_ptr = &__LAYOUT_BIN_END as *const u8;
            let len = end_ptr.offset_from(start_ptr) as usize;
            core::slice::from_raw_parts(start_ptr, len)
        };

        // Deserialize the binary data to LayoutNode using postcard
        let layout_node = from_bytes(layout_data)
            .map_err(|_| AppError::LayoutDeserialize)?;

        Ok(layout_node)
    }
}
