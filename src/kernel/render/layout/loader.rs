//! 布局加载器
//! 负责从生成的二进制数据中加载和反序列化布局

use crate::assets::generated_layouts::get_layout_data;
use crate::common::error::{AppError, Result};
use crate::kernel::render::layout::nodes::LayoutPool;
use postcard::from_bytes;

/// 布局加载器
pub struct LayoutLoader;

impl LayoutLoader {
    /// 创建新的布局加载器
    pub const fn new() -> Self {
        Self {}
    }

    /// 从生成的二进制数据加载布局
    pub fn load_layout(&self) -> Result<LayoutPool> {
        log::debug!("Loading layout data from binary");
        let layout_data = get_layout_data();
        log::debug!("Layout data size: {} bytes", layout_data.len());

        // 使用postcard反序列化布局数据
        let layout =
            from_bytes::<LayoutPool>(layout_data).map_err(|_| AppError::LayoutLoadFailed)?;
        log::info!("Layout loaded successfully");
        Ok(layout)
    }
}

/// 默认布局加载器
pub const DEFAULT_LOADER: LayoutLoader = LayoutLoader::new();
