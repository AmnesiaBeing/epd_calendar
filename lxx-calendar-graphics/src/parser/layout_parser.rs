//! 布局 JSON 解析器

extern crate alloc;

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use lxx_calendar_common::layout::{
    ConditionOp, LayoutDefinition, SeparatorStyle, TextAlign,
};
use serde_json::Value;

/// 解析错误类型
#[derive(Debug, Clone, thiserror::Error)]
pub enum ParseError {
    #[error("JSON 解析错误：{message}")]
    JsonParseError { message: String },
    #[error("布局验证错误：{message}")]
    LayoutValidationError { message: String },
    #[error("渲染错误：{message}")]
    RenderError { message: String },
}

/// 布局解析器
pub struct LayoutParser;

impl LayoutParser {
    /// 解析布局 JSON 字符串
    pub fn parse_layout(json_str: &str) -> Result<LayoutDefinition, ParseError> {
        serde_json::from_str(json_str).map_err(|e| ParseError::JsonParseError {
            message: e.to_string(),
        })
    }

    /// 解析数据 JSON 字符串为 LayoutData
    pub fn parse_data(json_str: &str) -> Result<crate::LayoutData, ParseError> {
        let value: Value = serde_json::from_str(json_str).map_err(|e| ParseError::JsonParseError {
            message: e.to_string(),
        })?;
        Self::value_to_layout_data(&value)
    }

    /// 从 JSON Value 转换为 LayoutData
    pub fn value_to_layout_data(value: &Value) -> Result<crate::LayoutData, ParseError> {
        use crate::LayoutData;
        
        let mut data = LayoutData::new();
        
        if let Value::Object(map) = value {
            for (key, val) in map {
                match val {
                    Value::String(s) => {
                        let _ = data.set_string(key, s);
                    }
                    Value::Number(n) => {
                        if let Some(i) = n.as_i64() {
                            let _ = data.set_i64(key, i);
                        } else if let Some(f) = n.as_f64() {
                            let _ = data.set_f64(key, f);
                        }
                    }
                    Value::Bool(b) => {
                        let _ = data.set_bool(key, *b);
                    }
                    Value::Array(arr) => {
                        let mut items = heapless::Vec::new();
                        for item in arr {
                            if let Ok(item_data) = Self::value_to_layout_data(item) {
                                let _ = items.push(item_data);
                            }
                        }
                        let _ = data.set_array(key, items);
                    }
                    Value::Object(_) => {
                        // 嵌套对象也作为 LayoutData 处理
                        if let Ok(nested) = Self::value_to_layout_data(val) {
                            let mut items = heapless::Vec::new();
                            let _ = items.push(nested);
                            let _ = data.set_array(key, items);
                        }
                    }
                    Value::Null => {}
                }
            }
        }
        
        Ok(data)
    }

    /// 从 JSON Value 中获取文本内容
    /// 支持 field 和 template 两种方式
    pub fn get_text_content(value: &Value, field: Option<&str>, template: Option<&str>) -> String {
        if let Some(tpl) = template {
            Self::apply_template(tpl, value)
        } else if let Some(f) = field {
            Self::get_field_string(value, f)
        } else {
            String::new()
        }
    }

    /// 应用模板到 JSON 值
    pub fn apply_template(template: &str, value: &Value) -> String {
        let mut result = template.to_string();

        if let Value::Object(map) = value {
            for (key, val) in map {
                if let Some(str_val) = val.as_str() {
                    result = result.replace(&format!("{{{}}}", key), str_val);
                } else if let Some(num_val) = val.as_i64() {
                    result = result.replace(&format!("{{{}}}", key), &num_val.to_string());
                } else if let Some(num_val) = val.as_u64() {
                    result = result.replace(&format!("{{{}}}", key), &num_val.to_string());
                }
            }
        }

        result
    }

    /// 从 JSON 值中获取字段字符串
    pub fn get_field_string(value: &Value, field: &str) -> String {
        value
            .get(field)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .or_else(|| value.get(field).map(|v| v.to_string()))
            .unwrap_or_default()
    }

    /// 从 JSON 值中获取字段整数
    pub fn get_field_i64(value: &Value, field: &str) -> Option<i64> {
        value.get(field).and_then(|v| v.as_i64())
    }

    /// 从 JSON 值中获取字段无符号整数
    pub fn get_field_u64(value: &Value, field: &str) -> Option<u64> {
        value.get(field).and_then(|v| v.as_u64())
    }

    /// 从 JSON 值中获取字段浮点数
    pub fn get_field_f64(value: &Value, field: &str) -> Option<f64> {
        value.get(field).and_then(|v| v.as_f64())
    }

    /// 从 JSON 值中获取字段数组
    pub fn get_field_array<'a>(value: &'a Value, field: &str) -> Option<&'a Vec<Value>> {
        value.get(field).and_then(|v| v.as_array())
    }

    /// 从 JSON 值中获取字段布尔值
    pub fn get_field_bool(value: &Value, field: &str) -> Option<bool> {
        value.get(field).and_then(|v| v.as_bool())
    }

    /// 评估条件
    pub fn evaluate_condition(
        op: &ConditionOp,
        field_value: Option<&Value>,
        compare_value: Option<&Value>,
    ) -> bool {
        match op {
            ConditionOp::Eq => {
                if let (Some(fv), Some(cv)) = (field_value, compare_value) {
                    return fv == cv;
                }
                false
            }
            ConditionOp::Gt => {
                if let (Some(fv), Some(cv)) = (field_value, compare_value) {
                    return Self::compare_values_greater(fv, cv);
                }
                false
            }
            ConditionOp::Lt => {
                if let (Some(fv), Some(cv)) = (field_value, compare_value) {
                    return Self::compare_values_less(fv, cv);
                }
                false
            }
            ConditionOp::Gte => {
                if let (Some(fv), Some(cv)) = (field_value, compare_value) {
                    return Self::compare_values_greater(fv, cv) || fv == cv;
                }
                false
            }
            ConditionOp::Lte => {
                if let (Some(fv), Some(cv)) = (field_value, compare_value) {
                    return Self::compare_values_less(fv, cv) || fv == cv;
                }
                false
            }
            ConditionOp::LenEq => {
                if let (Some(fv), Some(cv)) = (field_value, compare_value) {
                    if let (Some(len_f), Some(len_c)) =
                        (Self::get_length(fv), cv.as_u64())
                    {
                        return len_f == len_c;
                    }
                }
                false
            }
            ConditionOp::LenGt => {
                if let (Some(fv), Some(cv)) = (field_value, compare_value) {
                    if let (Some(len_f), Some(len_c)) =
                        (Self::get_length(fv), cv.as_u64())
                    {
                        return len_f > len_c;
                    }
                }
                false
            }
            ConditionOp::Exists => {
                field_value.is_some() && !matches!(field_value, Some(Value::Null))
            }
        }
    }

    /// 获取值的长度（数组或字符串）
    fn get_length(value: &Value) -> Option<u64> {
        if let Some(arr) = value.as_array() {
            Some(arr.len() as u64)
        } else if let Some(s) = value.as_str() {
            Some(s.len() as u64)
        } else {
            None
        }
    }

    /// 比较两个值的大小（大于）
    fn compare_values_greater(a: &Value, b: &Value) -> bool {
        match (a, b) {
            (Value::Number(an), Value::Number(bn)) => {
                an.as_f64().zip(bn.as_f64()).map(|(a, b)| a > b).unwrap_or(false)
            }
            (Value::String(as_), Value::String(bs)) => as_ > bs,
            _ => false,
        }
    }

    /// 比较两个值的大小（小于）
    fn compare_values_less(a: &Value, b: &Value) -> bool {
        match (a, b) {
            (Value::Number(an), Value::Number(bn)) => {
                an.as_f64().zip(bn.as_f64()).map(|(a, b)| a < b).unwrap_or(false)
            }
            (Value::String(as_), Value::String(bs)) => as_ < bs,
            _ => false,
        }
    }

    /// 解析对齐方式
    pub fn parse_text_align(s: &str) -> TextAlign {
        match s.to_lowercase().as_str() {
            "left" => TextAlign::Left,
            "center" => TextAlign::Center,
            "right" => TextAlign::Right,
            _ => TextAlign::Center,
        }
    }

    /// 解析分隔线样式
    pub fn parse_separator_style(s: &str) -> SeparatorStyle {
        match s.to_lowercase().as_str() {
            "solid" => SeparatorStyle::Solid,
            "dashed" => SeparatorStyle::Dashed,
            "short" => SeparatorStyle::Short,
            _ => SeparatorStyle::Solid,
        }
    }
}
