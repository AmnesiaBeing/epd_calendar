//! 条件评估和占位符替换
//! 实现对布局文件中条件表达式和变量占位符的解析和评估

use alloc::string::String;
use alloc::vec::Vec;

// 导入类型别名
use crate::kernel::data::types::{CacheKeyValueMap, DynamicValue};

use crate::common::error::{AppError, Result};
use crate::kernel::data::DataSourceRegistry;

use core::fmt::Write;

/// 表达式评估器
pub struct ExpressionEvaluator;

impl ExpressionEvaluator {
    /// 创建新的表达式评估器
    pub const fn new() -> Self {
        Self {}
    }

    /// 评估条件表达式（返回布尔值）
    pub fn evaluate_condition(
        &self,
        condition: &str,
        data: &DataSourceRegistry,
        cache: &CacheKeyValueMap,
    ) -> Result<bool> {
        log::debug!("Evaluating condition: '{}'", condition);
        // 简化实现：仅支持 变量 比较运算符 值 的格式
        // 完整实现需解析AST，此处适配嵌入式简化需求
        let condition = condition.trim();
        if condition.is_empty() {
            return Ok(true); // 空条件视为true
        }

        // 拆分表达式（如 "{a} == 10"）
        let parts: Vec<&str> = condition
            .split(|c| c == '>' || c == '<' || c == '=' || c == '!' || c == '&' || c == '|')
            .collect();
        if parts.len() < 2 {
            return Err(AppError::InvalidSyntax);
        }

        // 解析左值（变量）
        let left_part = parts[0].trim();
        let left_value = if left_part.starts_with('{') && left_part.ends_with('}') {
            let var_path = left_part.trim_matches(|c| c == '{' || c == '}');
            self.get_variable_value(data, cache, var_path)
                .map_err(|_| AppError::InvalidVariable)?
        } else {
            return Err(AppError::InvalidSyntax);
        };

        // 解析右值（常量）
        let right_part = parts[1].trim();
        let right_value = self.parse_constant(right_part)?;

        // 解析运算符
        let op = self.extract_operator(condition)?;

        // 执行比较
        self.compare_values(&left_value, op, &right_value)
    }

    /// 替换占位符并计算最终内容
    pub fn evaluate_content(
        &self,
        data: &DataSourceRegistry,
        cache: &CacheKeyValueMap,
        content: &str,
    ) -> Result<String> {
        log::debug!("Replacing placeholders in content: '{}'", content);
        let mut result = String::new();
        let mut chars = content.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '{' {
                // 开始解析占位符
                let mut placeholder = String::new(); // 为占位符字符串指定长度32
                let mut depth = 1;

                for c in chars.by_ref() {
                    match c {
                        '{' => depth += 1,
                        '}' => depth -= 1,
                        _ => {
                            if depth > 0 {
                                placeholder.push(c);
                            }
                        }
                    }

                    if depth == 0 {
                        // 占位符解析完成
                        let evaluated = self.evaluate_placeholder(data, cache, &placeholder)?;
                        result.push_str(&evaluated);
                        break;
                    }
                }

                if depth > 0 {
                    // 没有找到匹配的结束括号
                    return Err(AppError::LayoutPlaceholder);
                }
            } else {
                // 普通字符直接添加
                result.push(c);
            }
        }

        Ok(result)
    }

    /// 解析变量引用（处理嵌套≤2层）
    fn parse_variable(
        &self,
        chars: &mut core::iter::Peekable<core::str::Chars<'_>>,
        depth: u8,
    ) -> Result<String> {
        // 检查嵌套层级
        if depth > 2 {
            return Err(AppError::NestedTooDeep);
        }

        let mut var_path = String::new();
        while let Some(&c) = chars.peek() {
            if c == '}' {
                chars.next(); // 跳过闭合花括号
                return Ok(var_path);
            } else if c == '{' {
                // 嵌套变量
                chars.next(); // 跳过开括号
                let nested_var = self.parse_variable(chars, depth + 1)?;
                var_path.push_str(&nested_var);
            } else {
                // 检查花括号内是否有非法运算符
                if ['+', '-', '*', '/', '%', '?'].contains(&c) {
                    return Err(AppError::UnsupportedOp);
                }
                var_path.push(c);
                chars.next();
            }
        }

        Err(AppError::InvalidSyntax)
    }

    /// 解析常量值
    fn parse_constant(&self, s: &str) -> Result<DynamicValue> {
        if s.is_empty() {
            return Ok(DynamicValue::String(HeaplessString::new()));
        }

        // 布尔值
        if s == "true" {
            return Ok(DynamicValue::Boolean(true));
        }
        if s == "false" {
            return Ok(DynamicValue::Boolean(false));
        }

        // 整数
        if let Ok(i) = s.parse::<i32>() {
            return Ok(DynamicValue::Integer(i));
        }

        // 浮点数
        if let Ok(fl) = s.parse::<f32>() {
            return Ok(DynamicValue::Float(fl));
        }

        // 字符串（去除引号）
        let s_trimmed = s.trim_matches(|c| c == '"' || c == '\'');
        let mut str_val = HeaplessString::<CONTENT_LENGTH>::new();
        str_val
            .extend(s_trimmed.chars())
            .map_err(|_| AppError::TooLong)?;
        Ok(DynamicValue::String(str_val))
    }

    /// 提取运算符
    fn extract_operator(&self, expr: &str) -> Result<&str> {
        // 支持的运算符
        let ops = ["==", "!=", ">", "<", ">=", "<=", "&&", "||", "!"];
        for op in ops {
            if expr.contains(op) {
                return Ok(op);
            }
        }
        Err(AppError::UnsupportedOp)
    }

    /// 比较两个值
    fn compare_values(&self, left: &DynamicValue, op: &str, right: &DynamicValue) -> Result<bool> {
        match (left, op, right) {
            // 布尔比较
            (DynamicValue::Boolean(l), "==", DynamicValue::Boolean(r)) => Ok(l == r),
            (DynamicValue::Boolean(l), "!=", DynamicValue::Boolean(r)) => Ok(l != r),

            // 整数比较
            (DynamicValue::Integer(l), "==", DynamicValue::Integer(r)) => Ok(l == r),
            (DynamicValue::Integer(l), "!=", DynamicValue::Integer(r)) => Ok(l != r),
            (DynamicValue::Integer(l), ">", DynamicValue::Integer(r)) => Ok(l > r),
            (DynamicValue::Integer(l), "<", DynamicValue::Integer(r)) => Ok(l < r),
            (DynamicValue::Integer(l), ">=", DynamicValue::Integer(r)) => Ok(l >= r),
            (DynamicValue::Integer(l), "<=", DynamicValue::Integer(r)) => Ok(l <= r),

            // 浮点数比较
            (DynamicValue::Float(l), "==", DynamicValue::Float(r)) => Ok((l - r).abs() < 0.01),
            (DynamicValue::Float(l), "!=", DynamicValue::Float(r)) => Ok((l - r).abs() >= 0.01),
            (DynamicValue::Float(l), ">", DynamicValue::Float(r)) => Ok(l > r),
            (DynamicValue::Float(l), "<", DynamicValue::Float(r)) => Ok(l < r),

            // 字符串比较
            (DynamicValue::String(l), "==", DynamicValue::String(r)) => Ok(l == r),
            (DynamicValue::String(l), "!=", DynamicValue::String(r)) => Ok(l != r),

            // 存在性检查（空字符串）
            (DynamicValue::String(l), "!=", DynamicValue::String(r)) if r.is_empty() => {
                Ok(!l.is_empty())
            }
            (DynamicValue::String(l), "==", DynamicValue::String(r)) if r.is_empty() => {
                Ok(l.is_empty())
            }

            _ => Err(AppError::TypeMismatch),
        }
    }

    /// 评估单个占位符
    fn evaluate_placeholder(
        &self,
        data: &DataSourceRegistry,
        cache: &CacheKeyValueMap,
        placeholder: &str,
    ) -> Result<String> {
        log::debug!("Evaluating placeholder: '{}'", placeholder);
        self.get_variable_value(data, cache, placeholder.trim())
    }

    /// 获取变量值
    fn get_variable_value(
        &self,
        data: &DataSourceRegistry,
        cache: &CacheKeyValueMap,
        path: &str,
    ) -> Result<String> {
        // 使用数据源注册表获取变量值
        // 这里我们假设注册表支持同步访问缓存的数据
        match data.get_value_by_path_sync(cache, path) {
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
                        s.push_str(&str_val);
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
