//! 编译期使用的的极简校验函数

use super::*;

pub fn validate_content_placeholders(content: &str) -> Result<()> {
    // 校验占位符格式：{xxx.xxx.xxx}
    let placeholders: Vec<&str> = content
        .split('{')
        .skip(1)
        .map(|s| s.split('}').next().unwrap_or(""))
        .filter(|s| !s.is_empty())
        .collect();

    for placeholder in placeholders {
        // 占位符只能包含字母、数字、下划线、点
        if !placeholder
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '.' || c == '[' || c == ']')
        {
            return Err(anyhow::anyhow!(
                "占位符{}包含非法字符，仅允许字母/数字/下划线/点",
                placeholder
            ));
        }
    }

    Ok(())
}

/// 验证icon_id格式
pub fn validate_icon_id(icon_id: &str, config: &BuildConfig) -> Result<()> {
    // 长度校验
    if icon_id.len() > MAX_ICON_ID_LENGTH {
        return Err(anyhow::anyhow!(
            "icon_id长度超限: {} > {}",
            icon_id.len(),
            MAX_ICON_ID_LENGTH
        ));
    }

    // 格式校验：必须包含且仅包含1个冒号
    let parts: Vec<&str> = icon_id.split(':').collect();
    if parts.len() != 2 {
        return Err(anyhow::anyhow!(
            "icon_id格式错误，必须包含且仅包含1个冒号: {}",
            icon_id
        ));
    }

    let module = parts[0];

    // 前缀匹配校验
    let mut matched = false;

    // 检查图标分类配置
    for category in &config.icon_categories {
        if category.category == module {
            matched = true;
            break;
        }
    }

    // 检查天气图标
    if !matched && module == "weather" {
        matched = true;
    }

    if !matched {
        return Err(anyhow::anyhow!(
            "icon_id前缀'{}'未匹配任何图标模块配置",
            module
        ));
    }

    Ok(())
}

/// 验证表达式嵌套层级
pub fn validate_expression_nesting(expr: &str) -> Result<()> {
    let mut max_depth = 0;
    let mut current_depth = 0;

    for ch in expr.chars() {
        match ch {
            '{' => {
                current_depth += 1;
                if current_depth > max_depth {
                    max_depth = current_depth;
                }
                if max_depth > 2 {
                    return Err(anyhow::anyhow!("表达式嵌套层级超过2层: {}", expr));
                }
            }
            '}' => {
                if current_depth == 0 {
                    return Err(anyhow::anyhow!("表达式括号不匹配: {}", expr));
                }
                current_depth -= 1;
            }
            _ => {}
        }
    }

    if current_depth != 0 {
        return Err(anyhow::anyhow!("表达式括号不匹配: {}", expr));
    }

    Ok(())
}

/// 验证条件表达式语法
pub fn validate_condition_expression(condition: &str) -> Result<()> {
    // 长度校验
    if condition.len() > MAX_CONDITION_LENGTH {
        return Err(anyhow::anyhow!(
            "条件表达式长度超限: {} > {}",
            condition.len(),
            MAX_CONDITION_LENGTH
        ));
    }

    // 嵌套层级校验
    validate_expression_nesting(condition)?;

    // 花括号内禁止运算符校验
    let in_brace = condition
        .chars()
        .fold((false, false), |(in_brace, has_op), ch| match ch {
            '{' => (true, false),
            '}' => (false, false),
            '+' | '-' | '*' | '/' | '%' | '?' => (in_brace, in_brace || has_op),
            _ => (in_brace, has_op),
        });

    if in_brace.1 {
        return Err(anyhow::anyhow!(
            "条件表达式花括号内包含运算符: {}",
            condition
        ));
    }

    Ok(())
}
