//! JSON 布局解析器和模式加载器
//!
//! 负责从 JSON 字符串或 Flash 存储中加载和解析模式定义

extern crate alloc;

use heapless::Vec;

use super::types::ModeDefinition;
use lxx_calendar_common::{DataError, SystemError, SystemResult};

/// 模式加载器 - 管理已加载的模式定义
pub struct ModeLoader {
    /// 已加载的模式定义 (最多 16 个)
    modes: Vec<ModeDefinition, 16>,
}

impl ModeLoader {
    /// 创建新的模式加载器
    pub fn new() -> Self {
        Self {
            modes: Vec::new(),
        }
    }

    /// 从 JSON 字符串加载单个模式定义
    /// 
    /// 注意：在 no_std 环境下，此函数需要使用 serde-json-core 或类似的库
    /// 目前这个实现是占位符，实际使用时需要在 std 环境下解析后传递结果
    pub fn load_from_json(&mut self, _json_str: &str) -> SystemResult<ModeDefinition> {
        // TODO: 在 no_std 环境下实现 JSON 解析
        // 目前返回一个错误，提示使用其他方式加载
        Err(SystemError::DataError(DataError::ParseError))
    }

    /// 直接添加已解析的模式定义
    /// 
    /// 这是在 no_std 环境下的推荐用法：
    /// 1. 在构建时或 std 环境下解析 JSON
    /// 2. 直接使用此方法添加模式
    pub fn add_mode(&mut self, mode: ModeDefinition) -> SystemResult<()> {
        // 检查是否已存在相同 mode_id
        for existing in self.modes.iter() {
            if existing.mode_id.to_uppercase() == mode.mode_id.to_uppercase() {
                return Err(SystemError::DataError(DataError::Unknown));
            }
        }

        self.modes
            .push(mode)
            .map_err(|_| SystemError::ServiceError(lxx_calendar_common::ServiceError::OperationFailed))?;

        Ok(())
    }

    /// 从 Flash 加载模式定义
    pub async fn load_from_flash(&mut self, _flash: &mut impl lxx_calendar_common::storage::FlashDevice) -> SystemResult<usize> {
        // TODO: 实现 Flash 加载
        Ok(0)
    }

    /// 保存模式定义到 Flash
    pub async fn save_to_flash(&self, _flash: &mut impl lxx_calendar_common::storage::FlashDevice) -> SystemResult<()> {
        // TODO: 实现 Flash 保存
        Ok(())
    }

    /// 获取模式定义
    pub fn get_mode(&self, mode_id: &str) -> Option<&ModeDefinition> {
        let mode_id_upper = mode_id.to_uppercase();
        self.modes.iter().find(|m| m.mode_id.to_uppercase() == mode_id_upper)
    }

    /// 获取模式定义（可变引用）
    pub fn get_mode_mut(&mut self, mode_id: &str) -> Option<&mut ModeDefinition> {
        let mode_id_upper = mode_id.to_uppercase();
        self.modes.iter_mut().find(|m| m.mode_id.to_uppercase() == mode_id_upper)
    }

    /// 获取所有模式 ID
    pub fn get_mode_ids(&self) -> Vec<&str, 16> {
        let mut ids: Vec<&str, 16> = Vec::new();
        for mode in self.modes.iter() {
            let _ = ids.push(mode.mode_id.as_str());
        }
        ids
    }

    /// 获取已加载模式数量
    pub fn count(&self) -> usize {
        self.modes.len()
    }

    /// 检查是否包含指定模式
    pub fn contains(&self, mode_id: &str) -> bool {
        self.get_mode(mode_id).is_some()
    }

    /// 清除所有已加载的模式
    pub fn clear(&mut self) {
        self.modes.clear();
    }

    /// 加载内置模式（编译时定义）
    pub fn load_builtin_modes(&mut self) -> SystemResult<()> {
        #[cfg(feature = "builtin-modes")]
        {
            // TODO: 从编译生成的数据加载
        }

        Ok(())
    }
}

impl Default for ModeLoader {
    fn default() -> Self {
        Self::new()
    }
}
