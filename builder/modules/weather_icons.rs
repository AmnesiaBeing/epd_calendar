//! 天气图标处理模块

use crate::builder::config::BuildConfig;
use crate::builder::utils::file_utils;
use crate::builder::utils::{
    icon_renderer::{IconConfig, IconRenderer},
    progress::ProgressTracker,
};
use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs;

/// 天气图标尺寸常量
const WEATHER_ICON_SIZE: u32 = 64;

/// 天气图标数据结构
#[derive(Debug, Deserialize)]
pub struct WeatherIcon {
    pub icon_code: String,
    pub icon_name: String,
}

/// 构建天气图标数据
pub fn build(config: &BuildConfig, progress: &ProgressTracker) -> Result<()> {
    progress.update_progress(0, 4, "读取图标列表");
    let icons = load_icon_list(config)?;

    progress.update_progress(1, 4, "处理SVG图标");
    let (icon_data, icon_mapping) = process_weather_icons(config, &icons, progress)?;

    progress.update_progress(2, 4, "生成二进制文件");
    generate_icon_binary_files(config, &icon_data)?;

    progress.update_progress(3, 4, "生成索引文件");
    generate_weather_icons_rs(config, &icons, &icon_mapping)?;

    println!(
        "cargo:warning=  天气图标处理完成，共处理 {} 个图标",
        icon_mapping.len()
    );
    Ok(())
}

/// 加载图标列表
fn load_icon_list(config: &BuildConfig) -> Result<Vec<WeatherIcon>> {
    let icons_list_path = config.weather_icons_dir.join("icons-list.json");

    let icons_list_content = fs::read_to_string(&icons_list_path)
        .with_context(|| format!("读取图标列表失败: {}", icons_list_path.display()))?;

    let icons: Vec<WeatherIcon> =
        serde_json::from_str(&icons_list_content).context("解析图标列表失败")?;

    println!("cargo:warning=  找到 {} 个天气图标", icons.len());

    // 去重逻辑
    let mut unique_icons = Vec::new();
    let mut seen_names = std::collections::HashSet::new();

    for icon in icons {
        if seen_names.insert(icon.icon_name.clone()) {
            unique_icons.push(icon);
        } else {
            println!("cargo:warning=  跳过重复的图标名称: {}", icon.icon_name);
        }
    }

    println!("cargo:warning=  去重后图标数量: {}", unique_icons.len());
    Ok(unique_icons)
}

/// 处理天气图标
fn process_weather_icons(
    config: &BuildConfig,
    icons: &[WeatherIcon],
    progress: &ProgressTracker,
) -> Result<(Vec<u8>, BTreeMap<String, usize>)> {
    let icons_dir = config.weather_icons_dir.join("icons");
    let mut icon_data = Vec::new();
    let mut icon_mapping = BTreeMap::new();
    let mut preview_results = Vec::new(); // 用于存储预览数据

    for (index, icon) in icons.iter().enumerate() {
        let svg_path = icons_dir.join(format!("{}.svg", icon.icon_code));

        if !svg_path.exists() {
            println!("cargo:warning=  SVG文件不存在: {}", svg_path.display());
            continue;
        }

        // 使用通用图标渲染器渲染图标
        let icon_config = IconConfig {
            icon_size: WEATHER_ICON_SIZE,
            svg_path: svg_path.to_string_lossy().to_string(),
        };

        let result = IconRenderer::render_svg_icon(&icon_config)
            .with_context(|| format!("渲染图标失败: {}", icon.icon_code))?;

        // 记录图标数据位置
        let start_index = icon_data.len();
        icon_mapping.insert(icon.icon_code.clone(), start_index);

        // 添加图标数据
        icon_data.extend_from_slice(&result.bitmap_data);

        // 存储预览数据
        preview_results.push((icon.icon_code.clone(), result));

        // 显示进度
        if index % 10 == 0 {
            progress.update_progress(index, icons.len(), "处理天气图标");
        }
    }

    // 预览其中一个图标
    let (name, result) = preview_results.first().unwrap();
    IconRenderer::preview_icon(result, name, WEATHER_ICON_SIZE);

    Ok((icon_data, icon_mapping))
}

/// 生成图标二进制文件
fn generate_icon_binary_files(config: &BuildConfig, icon_data: &[u8]) -> Result<()> {
    let icons_bin_path = config.output_dir.join("weather_icons.bin");
    file_utils::write_file(&icons_bin_path, icon_data)?;

    println!(
        "cargo:warning=  天气图标二进制文件生成成功: {}",
        icons_bin_path.display()
    );

    Ok(())
}

/// 生成天气图标索引文件
fn generate_weather_icons_rs(
    config: &BuildConfig,
    icons: &[WeatherIcon],
    icon_mapping: &BTreeMap<String, usize>,
) -> Result<()> {
    let output_path = config.output_dir.join("weather_icons.rs");

    let content = generate_icons_rs_content(icons, icon_mapping, WEATHER_ICON_SIZE)?;
    file_utils::write_string_file(&output_path, &content)?;

    println!(
        "cargo:warning=  天气图标索引文件生成成功: {}",
        output_path.display()
    );
    Ok(())
}

/// 生成天气图标Rust文件内容
fn generate_icons_rs_content(
    icons: &[WeatherIcon],
    icon_mapping: &BTreeMap<String, usize>,
    icon_size: u32,
) -> Result<String> {
    let mut content = String::new();

    content.push_str("// 自动生成的天气图标数据文件\n");
    content.push_str("// 不要手动修改此文件\n\n");

    content.push_str("use embedded_graphics::{\n");
    content.push_str("    image::ImageRaw,\n");
    content.push_str("    prelude::*,\n");
    content.push_str("    pixelcolor::BinaryColor,\n");
    content.push_str("};\n\n");

    // 定义图标枚举 - 简化 trait 并允许非驼峰命名
    content.push_str("#[allow(non_camel_case_types)]\n");
    content.push_str("#[derive(Clone, Copy, PartialEq, Debug)]\n");
    content.push_str("pub enum WeatherIcon {\n");
    for icon in icons {
        // 将图标名称转换为有效的 Rust 标识符
        let variant_name = icon.icon_name.replace('-', "_").replace(' ', "_");
        content.push_str(&format!("    {},\n", variant_name));
    }
    content.push_str("}\n\n");

    // 计算每个图标的字节大小
    let bytes_per_icon = ((icon_size * icon_size) / 8) as usize;

    // 实现图标到索引的转换
    content.push_str("impl WeatherIcon {\n");
    content.push_str("    pub fn as_index(&self) -> usize {\n");
    content.push_str("        match self {\n");
    for (index, icon) in icons.iter().enumerate() {
        let variant_name = icon.icon_name.replace('-', "_").replace(' ', "_");
        content.push_str(&format!(
            "            WeatherIcon::{} => {},\n",
            variant_name, index
        ));
    }
    content.push_str("        }\n");
    content.push_str("    }\n\n");

    content.push_str("    pub fn from_code(code: &str) -> Option<Self> {\n");
    content.push_str("        match code {\n");
    for icon in icons {
        let variant_name = icon.icon_name.replace('-', "_").replace(' ', "_");
        content.push_str(&format!(
            "            \"{}\" => Some(WeatherIcon::{}),\n",
            icon.icon_code, variant_name
        ));
    }
    content.push_str("            _ => None,\n");
    content.push_str("        }\n");
    content.push_str("    }\n\n");

    content.push_str("    pub fn code(&self) -> &'static str {\n");
    content.push_str("        match self {\n");
    for icon in icons {
        let variant_name = icon.icon_name.replace('-', "_").replace(' ', "_");
        content.push_str(&format!(
            "            WeatherIcon::{} => \"{}\",\n",
            variant_name, icon.icon_code
        ));
    }
    content.push_str("        }\n");
    content.push_str("    }\n\n");

    content.push_str("    pub fn name(&self) -> &'static str {\n");
    content.push_str("        match self {\n");
    for icon in icons {
        let variant_name = icon.icon_name.replace('-', "_").replace(' ', "_");
        content.push_str(&format!(
            "            WeatherIcon::{} => \"{}\",\n",
            variant_name, icon.icon_name
        ));
    }
    content.push_str("        }\n");
    content.push_str("    }\n");
    content.push_str("}\n\n");

    // 定义图标数据
    content.push_str(&format!(
        "pub const WEATHER_ICON_DATA: &[u8] = include_bytes!(\"weather_icons.bin\");\n\n"
    ));

    content.push_str(&format!(
        "pub const WEATHER_ICON_SIZE: u32 = {};\n",
        icon_size
    ));
    content.push_str(&format!(
        "pub const WEATHER_ICON_COUNT: usize = {};\n",
        icons.len()
    ));

    // 生成图标索引数组
    content.push_str("pub const WEATHER_ICON_INDICES: &[usize] = &[\n");
    for icon in icons {
        if let Some(&start_index) = icon_mapping.get(&icon.icon_code) {
            content.push_str(&format!(
                "    {}, // {} ({})\n",
                start_index, icon.icon_name, icon.icon_code
            ));
        }
    }
    content.push_str("];\n\n");

    // 实用函数
    content.push_str("pub fn get_icon_data(icon: WeatherIcon) -> &'static [u8] {\n");
    content.push_str(&format!(
        "    let start = WEATHER_ICON_INDICES[icon.as_index()];\n"
    ));
    content.push_str(&format!(
        "    let end = start + {}; // {}x{} / 8 = {} bytes per icon\n",
        bytes_per_icon, icon_size, icon_size, bytes_per_icon
    ));
    content.push_str("    &WEATHER_ICON_DATA[start..end]\n");
    content.push_str("}\n\n");

    // 添加便捷函数来创建 ImageRaw
    content.push_str(
        "pub fn get_icon_image_raw(icon: WeatherIcon) -> ImageRaw<'static, BinaryColor> {\n",
    );
    content.push_str(&format!(
        "    ImageRaw::new(get_icon_data(icon), {})\n",
        icon_size
    ));
    content.push_str("}\n");

    // 添加便捷函数获取所有可用图标
    content.push_str("pub fn get_all_weather_icons() -> &'static [WeatherIcon] {\n");
    content.push_str("    &[\n");
    for icon in icons {
        let variant_name = icon.icon_name.replace('-', "_").replace(' ', "_");
        content.push_str(&format!("        WeatherIcon::{},\n", variant_name));
    }
    content.push_str("    ]\n");
    content.push_str("}\n");

    Ok(content)
}
