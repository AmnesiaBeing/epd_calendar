//! 编译期使用的的极简校验函数

use super::*;

/// 编译期+运行时通用的权重校验
pub fn validate_weight(weight: &f32) -> Result<()> {
    if *weight < MIN_WEIGHT || *weight > MAX_WEIGHT {
        return Err(anyhow::anyhow!(
            "权重超限: {} (需{}~{})",
            weight,
            MIN_WEIGHT,
            MAX_WEIGHT
        ));
    }
    Ok(())
}

/// 编译期+运行时通用的厚度校验
pub fn validate_thickness(thickness: &u16) -> Result<()> {
    if *thickness < MIN_THICKNESS || *thickness > MAX_THICKNESS {
        return Err(anyhow::anyhow!(
            "厚度超限: {} (需{}~{})",
            thickness,
            MIN_THICKNESS,
            MAX_THICKNESS
        ));
    }
    Ok(())
}

/// 修正权重值（规则4.6）
pub fn normalize_weight(weight: f32) -> f32 {
    if weight <= 0.0 {
        MIN_WEIGHT
    } else if weight > MAX_WEIGHT {
        MAX_WEIGHT
    } else {
        weight
    }
}

/// 修正描边/线宽（规则4.7）
pub fn normalize_thickness(thickness: u16) -> u16 {
    if thickness < MIN_THICKNESS {
        MIN_THICKNESS
    } else if thickness > MAX_THICKNESS {
        MAX_THICKNESS
    } else {
        thickness
    }
}

pub fn validate_font_size(font_size: &str, config: &BuildConfig) -> Result<String> {
    let lower_font = font_size.to_lowercase();
    let valid = config
        .font_size_configs
        .iter()
        .any(|fs| fs.name.to_lowercase() == lower_font);

    if valid {
        Ok(font_size.to_string())
    } else {
        let valid_sizes: Vec<&String> =
            config.font_size_configs.iter().map(|fs| &fs.name).collect();
        Err(anyhow::anyhow!(
            "字体尺寸{}无效，合法值：{:?}",
            font_size,
            valid_sizes
        ))
    }
}

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

pub fn validate_condition_syntax(condition: &str) -> Result<()> {
    // 简易条件表达式语法校验：仅允许 {xxx}、==、!=、&&、||、''、数字
    let allowed_chars = |c: char| {
        c.is_alphanumeric()
            || c == '_'
            || c == '.'
            || c == '{'
            || c == '}'
            || c == '='
            || c == '!'
            || c == '&'
            || c == '|'
            || c == '\''
            || c == ' '
            || c == '<'
            || c == '>'
    };

    if !condition.chars().all(allowed_chars) {
        return Err(anyhow::anyhow!("条件表达式包含非法字符: {}", condition));
    }

    // 校验括号匹配
    let open_count = condition.chars().filter(|&c| c == '{').count();
    let close_count = condition.chars().filter(|&c| c == '}').count();
    if open_count != close_count {
        return Err(anyhow::anyhow!("条件表达式占位符括号不匹配: {}", condition));
    }

    Ok(())
}

pub fn validate_coordinate(
    coord: &[u16; 2],
    node_type: &str,
    node_id: &str,
    is_absolute: bool,
) -> Result<()> {
    let max_x = if is_absolute {
        SCREEN_WIDTH
    } else {
        MAX_DIMENSION
    };
    let max_y = if is_absolute {
        SCREEN_HEIGHT
    } else {
        MAX_DIMENSION
    };

    if coord[0] > max_x || coord[1] > max_y {
        Err(anyhow::anyhow!(
            "{} {}坐标超限: [{:?}] (绝对定位: {}, 最大[{}/{}])",
            node_type,
            node_id,
            coord,
            is_absolute,
            max_x,
            max_y
        ))
    } else {
        Ok(())
    }
}

// ==================== 布局校验器（新增） ====================
#[derive(Debug, Default)]
pub struct LayoutValidationResult {
    pub warnings: Vec<String>,
}

pub struct LayoutValidator;

impl LayoutValidator {
    pub fn validate(
        yaml_node: &YamlLayoutNode,
        config: &BuildConfig,
    ) -> Result<LayoutValidationResult> {
        let mut result = LayoutValidationResult::default();
        Self::validate_node(yaml_node, &mut result, 0, config)?;
        Ok(result)
    }

    fn validate_node(
        yaml_node: &YamlLayoutNode,
        result: &mut LayoutValidationResult,
        nest_level: usize,
        config: &BuildConfig,
    ) -> Result<()> {
        // 嵌套层级警告
        if nest_level > MAX_NEST_LEVEL {
            result.warnings.push(format!(
                "节点嵌套层级超过上限: {} (最大{})",
                nest_level, MAX_NEST_LEVEL
            ));
        }

        match yaml_node {
            YamlLayoutNode::Container(yaml) => {
                // 子节点数量警告
                if yaml.children.len() > MAX_CHILDREN_COUNT / 2 {
                    result.warnings.push(format!(
                        "容器{}子节点数量较多: {} (最大{})",
                        yaml.id,
                        yaml.children.len(),
                        MAX_CHILDREN_COUNT
                    ));
                }

                // 递归校验子节点
                for child in &yaml.children {
                    Self::validate_node(&child.node, result, nest_level + 1, config)?;
                }
            }
            YamlLayoutNode::Text(yaml) => {
                // 文本内容过长警告
                if yaml.content.len() > MAX_CONTENT_LENGTH {
                    result.warnings.push(format!(
                        "文本{}内容过长: {}字符 (最大{})",
                        yaml.id,
                        yaml.content.len(),
                        MAX_CONTENT_LENGTH
                    ));
                }
            }
            YamlLayoutNode::Icon(yaml) => {
                // 图标ID格式校验（非错误，仅警告）
                if !yaml.icon_id.contains(':') {
                    result.warnings.push(format!(
                        "图标{}ID格式建议为{{模块}}:{{键}}: {}",
                        yaml.id, yaml.icon_id
                    ));
                }
            }
            YamlLayoutNode::Line(yaml) => {
                // 线条长度警告
                let length = (((yaml.end[0] - yaml.start[0]) as f32).powi(2)
                    + ((yaml.end[1] - yaml.start[1]) as f32).powi(2))
                .sqrt();
                if length < 5.0 {
                    result.warnings.push(format!(
                        "线条{}长度过短: {:.1}px (建议≥5px)",
                        yaml.id, length
                    ));
                }
            }
            YamlLayoutNode::Rectangle(yaml) => {
                // 矩形尺寸警告
                if yaml.width < 5 || yaml.height < 5 {
                    result.warnings.push(format!(
                        "矩形{}尺寸过小: {}x{} (建议≥5x5)",
                        yaml.id, yaml.width, yaml.height
                    ));
                }
            }
            YamlLayoutNode::Circle(yaml) => {
                // 圆形半径警告
                if yaml.radius < 5 {
                    result.warnings.push(format!(
                        "圆形{}半径过小: {}px (建议≥5px)",
                        yaml.id, yaml.radius
                    ));
                }
            }
        }

        Ok(())
    }
}
