//! 渲染上下文
//! 封装渲染过程中的状态和通用辅助方法

use alloc::string::String;

use crate::common::error::{AppError, Result};
use crate::kernel::data::{DataSourceRegistry, types::CacheKeyValueMap};
use crate::kernel::render::layout::evaluator::{DEFAULT_EVALUATOR, ExpressionEvaluator};
use crate::kernel::render::layout::nodes::{LayoutPool, SCREEN_HEIGHT, SCREEN_WIDTH};

/// 渲染上下文（无可变状态，避免借用冲突）
pub struct RenderContext<'a> {
    // 数据源注册表（用于占位符替换/条件评估）
    pub data_source_registry: &'a DataSourceRegistry,
    // 缓存键值映射
    pub cache: &'a CacheKeyValueMap,
    // 布局池引用（全局布局数据）
    pub layout_pool: &'a LayoutPool,
    // 表达式评估器（复用避免重复创建）
    evaluator: &'a ExpressionEvaluator,
    // 父容器的绝对坐标偏移（用于相对定位计算）
    parent_offset: [i32; 2],
}

impl<'a> RenderContext<'a> {
    /// 创建新的渲染上下文
    pub fn new(
        data_source_registry: &'a DataSourceRegistry,
        cache: &'a CacheKeyValueMap,
        layout_pool: &'a LayoutPool,
    ) -> Self {
        Self {
            data_source_registry,
            cache,
            layout_pool,
            evaluator: &DEFAULT_EVALUATOR,
            parent_offset: [0, 0], // 根节点偏移为0
        }
    }

    /// 评估节点显示条件
    pub fn evaluate_condition(&self, condition: &str) -> Result<bool> {
        self.evaluator
            .evaluate_condition(condition, self.data_source_registry)
    }

    /// 替换内容中的占位符
    pub fn replace_placeholders(&self, content: &str) -> Result<String> {
        self.evaluator
            .replace_placeholders(self.data_source_registry, self.cache, content)
    }

    /// 计算相对坐标转绝对坐标（考虑父容器偏移）
    pub fn to_absolute_coord(&self, coord: [u16; 2]) -> Result<[u16; 2]> {
        let x = (coord[0] as i32 + self.parent_offset[0]) as u16;
        let y = (coord[1] as i32 + self.parent_offset[1]) as u16;

        // 边界校验
        if x > SCREEN_WIDTH || y > SCREEN_HEIGHT {
            log::error!(
                "坐标超出屏幕范围: [{}, {}] (屏幕尺寸: {}x{})",
                x,
                y,
                SCREEN_WIDTH,
                SCREEN_HEIGHT
            );
            return Err(AppError::RenderError);
        }

        Ok([x, y])
    }

    /// 计算相对矩形转绝对矩形（考虑父容器偏移）
    pub fn to_absolute_rect(&self, rect: [u16; 4]) -> Result<[u16; 4]> {
        let [x, y, w, h] = rect;
        let [abs_x, abs_y] = self.to_absolute_coord([x, y])?;

        // 宽度/高度边界校验
        if abs_x + w > SCREEN_WIDTH || abs_y + h > SCREEN_HEIGHT {
            log::error!(
                "矩形超出屏幕范围: [{}, {}, {}, {}] (绝对坐标: [{}, {}, {}, {}])",
                x,
                y,
                w,
                h,
                abs_x,
                abs_y,
                w,
                h
            );
            return Err(AppError::RenderError);
        }

        Ok([abs_x, abs_y, w, h])
    }

    /// 创建子上下文（用于容器子节点渲染，更新父偏移）
    pub fn create_child_context(&self, child_offset: [i32; 2]) -> Self {
        Self {
            data_source_registry: self.data_source_registry,
            cache: self.cache,
            layout_pool: self.layout_pool,
            evaluator: self.evaluator,
            parent_offset: [
                self.parent_offset[0] + child_offset[0],
                self.parent_offset[1] + child_offset[1],
            ],
        }
    }
}
