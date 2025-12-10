//! 条件评估和占位符替换
//! 实现对布局文件中条件表达式和变量占位符的解析和评估

use crate::common::error::{AppError, Result};
use crate::kernel::data::DataSourceRegistry;
use crate::kernel::data::types::DynamicValue;
use crate::kernel::render::layout::nodes::*;

use core::fmt::Write;
use heapless::String;

/// 表达式评估器
pub struct ExpressionEvaluator;

impl ExpressionEvaluator {
    /// 创建新的表达式评估器
    pub const fn new() -> Self {
        Self {}
    }

    /// 评估条件表达式
    pub fn evaluate_condition(&self, condition: &str, _data: &DataSourceRegistry) -> Result<bool> {
        log::debug!("Evaluating condition: '{}'", condition);
        if condition.is_empty() {
            log::debug!("Empty condition, returning true");
            return Ok(true);
        }

        // 简单实现：仅支持变量引用和基本比较
        // TODO: 实现完整的表达式解析器
        Ok(true)
    }

    /// 替换占位符并计算最终内容
    pub fn replace_placeholders(
        &self,
        content: &str,
        data: &DataSourceRegistry,
    ) -> Result<String<MAX_CONTENT_LENGTH>> {
        log::debug!("Replacing placeholders in content: '{}'", content);
        let mut result = String::new();
        let mut chars = content.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '{' {
                // 开始解析占位符
                let mut placeholder = String::<32>::new(); // 为占位符字符串指定长度32
                let mut depth = 1;

                while let Some(c) = chars.next() {
                    match c {
                        '{' => depth += 1,
                        '}' => depth -= 1,
                        _ => {
                            if depth > 0 {
                                placeholder
                                    .push(c)
                                    .map_err(|_| AppError::LayoutPlaceholder)?;
                            }
                        }
                    }

                    if depth == 0 {
                        // 占位符解析完成
                        let evaluated = self.evaluate_placeholder(&placeholder, data)?;
                        result
                            .push_str(&evaluated)
                            .map_err(|_| AppError::LayoutPlaceholder)?;
                        break;
                    }
                }

                if depth > 0 {
                    // 没有找到匹配的结束括号
                    return Err(AppError::LayoutPlaceholder);
                }
            } else {
                // 普通字符直接添加
                result.push(c).map_err(|_| AppError::LayoutPlaceholder)?;
            }
        }

        Ok(result)
    }

    /// 评估单个占位符
    fn evaluate_placeholder(
        &self,
        placeholder: &str,
        data: &DataSourceRegistry,
    ) -> Result<String<MAX_CONTENT_LENGTH>> {
        log::debug!("Evaluating placeholder: '{}'", placeholder);
        // 简单实现：仅支持直接变量引用
        // TODO: 实现完整的表达式解析，包括运算符、函数等
        self.get_variable_value(placeholder.trim(), data)
    }

    /// 获取变量值
    fn get_variable_value(
        &self,
        path: &str,
        data: &DataSourceRegistry,
    ) -> Result<String<MAX_CONTENT_LENGTH>> {
        // 使用数据源注册表获取变量值
        // 这里我们假设注册表支持同步访问缓存的数据
        match data.get_cached_value(path) {
            Ok(dynamic_value) => {
                let mut s = String::new();
                match dynamic_value {
                    DynamicValue::Boolean(b) => {
                        write!(&mut s, "{}", b).map_err(|_| AppError::LayoutPlaceholder)?;
                    }
                    DynamicValue::Integer(i) => {
                        write!(&mut s, "{}", i).map_err(|_| AppError::LayoutPlaceholder)?;
                    }
                    DynamicValue::Float(f) => {
                        write!(&mut s, "{:.1}", f).map_err(|_| AppError::LayoutPlaceholder)?;
                    }
                    DynamicValue::String(str_val) => {
                        s.push_str(&str_val)
                            .map_err(|_| AppError::LayoutPlaceholder)?;
                    }
                }
                Ok(s)
            }
            Err(_) => Err(AppError::LayoutPlaceholder),
        }
    }
}

/// 默认表达式评估器
pub const DEFAULT_EVALUATOR: ExpressionEvaluator = ExpressionEvaluator::new();
