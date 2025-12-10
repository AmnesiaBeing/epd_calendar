//! 布局加载器
//! 负责从生成的二进制数据中加载和反序列化布局

use crate::assets::generated_layouts::get_layout_data;
use crate::kernel::render::layout::nodes::LayoutNode;
use postcard::from_bytes;

/// 布局加载错误
#[derive(Debug, PartialEq, Eq)]
pub enum LayoutLoadError {
    /// 反序列化失败
    DeserializationError,
    /// 数据为空
    EmptyData,
    /// 数据格式错误
    FormatError,
}

/// 布局加载器
pub struct LayoutLoader {
    // 可以添加缓存或其他状态
}

impl LayoutLoader {
    /// 创建新的布局加载器
    pub const fn new() -> Self {
        Self {}
    }

    /// 从生成的二进制数据加载布局
    pub fn load_layout(&self) -> Result<LayoutNode, LayoutLoadError> {
        let layout_data = get_layout_data();

        // 使用postcard反序列化布局数据
        from_bytes::<LayoutNode>(layout_data).map_err(|_| LayoutLoadError::DeserializationError)
    }
}

/// 默认布局加载器
pub const DEFAULT_LOADER: LayoutLoader = LayoutLoader::new();
