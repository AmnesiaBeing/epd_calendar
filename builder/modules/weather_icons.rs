//! 天气图标处理模块

use crate::builder::config::BuildConfig;
use crate::builder::utils::{file_utils, progress::ProgressTracker};
use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::HashSet;
use std::fs;

/// 天气图标尺寸常量
const WEATHER_ICON_WIDTH: u32 = 64;
const WEATHER_ICON_HEIGHT: u32 = 64;
const WEATHER_ICON_BYTES_PER_ICON: u32 = (WEATHER_ICON_WIDTH * WEATHER_ICON_HEIGHT) / 8; // 512

/// 原始图标数据结构
#[derive(Debug, Deserialize, Clone)]
pub struct WeatherIconInfo {
    pub icon_code: String,
    pub icon_name: String,
}

/// 处理后的图标信息
#[derive(Debug, Clone)]
struct ProcessedIconInfo {
    pub icon_code: String,
    pub variant_name: String, // 唯一的Rust枚举变体名
}

/// 构建天气图标数据
pub fn build(config: &BuildConfig, progress: &ProgressTracker) -> Result<()> {
    progress.update_progress(0, 5, "读取图标列表");
    let icons = load_icon_list(config)?;

    progress.update_progress(1, 5, "过滤-fill图标");
    let filtered_icons = filter_fill_icons(&icons);

    progress.update_progress(2, 5, "处理图标名称");
    let processed_icons = process_icon_names(&filtered_icons)?;

    progress.update_progress(3, 5, "处理SVG图标");
    let icon_data = process_icons_to_bitmap(config, &processed_icons, progress)?;

    progress.update_progress(4, 5, "生成数据文件");
    generate_compact_icon_files(config, &processed_icons, &icon_data)?;

    println!(
        "cargo:warning=  天气图标处理完成，共处理 {} 个图标",
        processed_icons.len()
    );

    Ok(())
}

/// 加载图标列表
fn load_icon_list(config: &BuildConfig) -> Result<Vec<WeatherIconInfo>> {
    let icons_list_path = config.weather_icons_dir.join("icons-list.json");
    let content = fs::read_to_string(&icons_list_path)
        .with_context(|| format!("读取图标列表失败: {}", icons_list_path.display()))?;

    let icons: Vec<WeatherIconInfo> = serde_json::from_str(&content).context("解析图标列表失败")?;

    println!("cargo:warning=  找到 {} 个天气图标", icons.len());
    Ok(icons)
}

/// 过滤掉-fill图标
fn filter_fill_icons(icons: &[WeatherIconInfo]) -> Vec<WeatherIconInfo> {
    let mut filtered = Vec::new();
    let mut seen_codes = HashSet::new();

    for icon in icons {
        if icon.icon_code.ends_with("-fill") {
            // 跳过-fill图标
            continue;
        }

        // 检查是否有对应的-fill版本
        let fill_code = format!("{}-fill", icon.icon_code);
        let has_fill_version = icons.iter().any(|i| i.icon_code == fill_code);

        if has_fill_version {
            // 有-fill版本，保留非fill版本
            if seen_codes.insert(icon.icon_code.clone()) {
                filtered.push(icon.clone());
            }
        } else {
            // 没有-fill版本，直接保留
            if seen_codes.insert(icon.icon_code.clone()) {
                filtered.push(icon.clone());
            }
        }
    }

    println!("cargo:warning=  过滤后图标数量: {}", filtered.len());

    // 按图标代码排序
    filtered.sort_by(|a, b| a.icon_code.cmp(&b.icon_code));

    filtered
}

/// 处理图标名称，转换为唯一的Rust枚举变体名
fn process_icon_names(icons: &[WeatherIconInfo]) -> Result<Vec<ProcessedIconInfo>> {
    let mut processed = Vec::new();
    let mut seen_variants = HashSet::new();
    let mut seen_codes = HashSet::new();

    for icon in icons {
        // 检查图标代码是否重复
        if !seen_codes.insert(icon.icon_code.clone()) {
            println!("cargo:warning=  跳过重复图标代码: {}", icon.icon_code);
            continue;
        }

        // 转换图标名称为有效的Rust枚举变体名
        let variant_name = convert_to_variant_name(&icon.icon_name);

        // 确保变体名唯一
        let unique_variant_name = make_unique_variant_name(variant_name, &mut seen_variants);

        processed.push(ProcessedIconInfo {
            icon_code: icon.icon_code.clone(),
            variant_name: unique_variant_name,
        });
    }

    println!("cargo:warning=  处理名称后图标数量: {}", processed.len());
    Ok(processed)
}

/// 转换图标名称为合法的Rust标识符
fn convert_to_variant_name(icon_name: &str) -> String {
    let mut variant_name = String::new();
    let mut last_was_underscore = false;

    for c in icon_name.chars() {
        if c.is_ascii_alphanumeric() {
            variant_name.push(c.to_ascii_uppercase());
            last_was_underscore = false;
        } else if c == ' '
            || c == '-'
            || c == '_'
            || c == '.'
            || c == '('
            || c == ')'
            || c == '（'
            || c == '）'
        {
            if !last_was_underscore && !variant_name.is_empty() {
                variant_name.push('_');
                last_was_underscore = true;
            }
        }
        // 跳过其他字符
    }

    // 去掉尾部的下划线
    variant_name = variant_name.trim_end_matches('_').to_string();

    // 如果名称为空，使用默认名称
    if variant_name.is_empty() {
        variant_name = "UNKNOWN".to_string();
    }

    // 确保以字母开头
    if !variant_name
        .chars()
        .next()
        .map_or(false, |c| c.is_ascii_alphabetic())
    {
        variant_name = format!("ICON_{}", variant_name);
    }

    variant_name
}

/// 确保变体名唯一
fn make_unique_variant_name(base_name: String, seen_variants: &mut HashSet<String>) -> String {
    let mut variant_name = base_name;
    let original_name = variant_name.clone();
    let mut counter = 1;

    while seen_variants.contains(&variant_name) {
        counter += 1;
        variant_name = format!("{}_{}", original_name, counter);
    }

    seen_variants.insert(variant_name.clone());
    variant_name
}

/// 处理图标为位图数据
fn process_icons_to_bitmap(
    config: &BuildConfig,
    icons: &[ProcessedIconInfo],
    progress: &ProgressTracker,
) -> Result<Vec<u8>> {
    let icons_dir = config.weather_icons_dir.join("icons");
    let mut all_bitmap_data = Vec::new();

    // 预分配空间
    let total_size = icons.len() * WEATHER_ICON_BYTES_PER_ICON as usize;
    all_bitmap_data.reserve(total_size);

    for (index, icon) in icons.iter().enumerate() {
        let svg_path = icons_dir.join(format!("{}.svg", icon.icon_code));

        if !svg_path.exists() {
            println!(
                "cargo:warning=  图标文件不存在，使用空白图标: {}",
                icon.icon_code
            );
            all_bitmap_data.extend(std::iter::repeat(0).take(WEATHER_ICON_BYTES_PER_ICON as usize));
            continue;
        }

        let icon_config = crate::builder::utils::icon_renderer::IconConfig {
            target_width: WEATHER_ICON_WIDTH,
            target_height: WEATHER_ICON_HEIGHT,
            svg_path: svg_path.to_string_lossy().to_string(),
        };

        let result =
            crate::builder::utils::icon_renderer::IconRenderer::render_svg_icon(&icon_config)
                .with_context(|| format!("渲染图标失败: {}", icon.icon_code))?;

        // 验证数据大小
        if result.bitmap_data.len() != WEATHER_ICON_BYTES_PER_ICON as usize {
            return Err(anyhow::anyhow!(
                "图标 {} 数据大小错误: 期望 {} 字节，实际 {} 字节",
                icon.icon_code,
                WEATHER_ICON_BYTES_PER_ICON,
                result.bitmap_data.len()
            ));
        }

        all_bitmap_data.extend_from_slice(&result.bitmap_data);

        if index % 10 == 0 || index == icons.len() - 1 {
            progress.update_progress(index + 1, icons.len(), "处理图标");
        }
    }

    if all_bitmap_data.len() != total_size {
        return Err(anyhow::anyhow!(
            "图标数据总大小错误: 期望 {} 字节，实际 {} 字节",
            total_size,
            all_bitmap_data.len()
        ));
    }

    Ok(all_bitmap_data)
}

/// 生成紧凑的图标文件
fn generate_compact_icon_files(
    config: &BuildConfig,
    icons: &[ProcessedIconInfo],
    icon_data: &[u8],
) -> Result<()> {
    let bin_path = config.output_dir.join("generated_weather_icons.bin");
    file_utils::write_file(&bin_path, icon_data)?;
    println!(
        "cargo:warning=  生成二进制文件: {} ({}KB)",
        bin_path.display(),
        icon_data.len() / 1024
    );

    let rs_path = config.output_dir.join("generated_weather_icons.rs");
    let content = generate_compact_icons_rs(icons)?;
    file_utils::write_string_file(&rs_path, &content)?;
    println!("cargo:warning=  生成Rust源文件: {}", rs_path.display());

    Ok(())
}

/// 生成紧凑的图标Rust源文件
fn generate_compact_icons_rs(icons: &[ProcessedIconInfo]) -> Result<String> {
    let mut content = String::new();

    content.push_str("//! 自动生成的天气图标数据（紧凑版）\n");
    content.push_str("//! 不要手动修改此文件\n\n");
    content.push_str("#![allow(dead_code, non_camel_case_types, non_upper_case_globals)]\n\n");

    // ========== 1. 常量定义 ==========
    content.push_str("// ==================== 图标常量 ====================\n\n");
    content.push_str(&format!(
        "pub const WEATHER_ICON_WIDTH: u8 = {};\n",
        WEATHER_ICON_WIDTH
    ));
    content.push_str(&format!(
        "pub const WEATHER_ICON_HEIGHT: u8 = {};\n",
        WEATHER_ICON_HEIGHT
    ));
    content.push_str(&format!(
        "pub const WEATHER_ICON_BYTES_PER_ICON: usize = {};\n",
        WEATHER_ICON_BYTES_PER_ICON
    ));
    content.push_str(&format!(
        "pub const WEATHER_ICON_COUNT: usize = {};\n\n",
        icons.len()
    ));

    // ========== 2. 图标枚举 ==========
    content.push_str("// ==================== 图标枚举 ====================\n\n");
    content.push_str("#[repr(u16)]\n");
    content.push_str("#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]\n");
    content.push_str("pub enum WeatherIcon {\n");

    for (index, icon) in icons.iter().enumerate() {
        content.push_str(&format!("    {} = {},\n", icon.variant_name, index));
    }
    content.push_str("}\n\n");

    // ========== 3. 位图数据 ==========
    content.push_str("// ==================== 位图数据 ====================\n\n");
    content.push_str(
        "pub const WEATHER_ICON_DATA: &[u8] = include_bytes!(\"generated_weather_icons.bin\");\n\n",
    );

    // ========== 4. 实现方法 ==========
    content.push_str("// ==================== 实现方法 ====================\n\n");
    content.push_str("impl WeatherIcon {\n");

    // 计算偏移量（使用u16）
    content.push_str("    /// 获取图标在位图数据中的偏移量\n");
    content.push_str("    pub fn offset(&self) -> usize {\n");
    content.push_str("        (*self as u16 as usize) * WEATHER_ICON_BYTES_PER_ICON\n");
    content.push_str("    }\n");

    content.push_str("}\n\n");

    // ========== 5. 主要访问函数 ==========
    content.push_str("// ==================== 访问函数 ====================\n\n");

    // 从代码获取图标
    content.push_str("/// 从图标代码获取图标\n");
    content.push_str("pub fn get_weather_icon_from_code(code: &str) -> Option<WeatherIcon> {\n");
    content.push_str("    match code {\n");

    for icon in icons {
        content.push_str(&format!(
            "        \"{}\" => Some(WeatherIcon::{}),\n",
            icon.icon_code, icon.variant_name
        ));
    }
    content.push_str("        _ => None,\n");
    content.push_str("    }\n");
    content.push_str("}\n\n");

    // 获取图标数据
    content.push_str("/// 获取天气图标的位图数据\n");
    content.push_str("pub fn get_weather_icon_data(icon: WeatherIcon) -> &'static [u8] {\n");
    content.push_str("    let start = icon.offset();\n");
    content.push_str("    let end = start + WEATHER_ICON_BYTES_PER_ICON;\n");
    content.push_str("    &WEATHER_ICON_DATA[start..end]\n");
    content.push_str("}\n");

    Ok(content)
}
