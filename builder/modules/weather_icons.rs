//! 天气图标处理模块

use crate::builder::config::BuildConfig;
use crate::builder::utils::{self, ProgressTracker};
use anyhow::{Context, Result, anyhow};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs;

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
    generate_weather_icons_rs(config, &icons)?;

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
) -> Result<(Vec<u8>, BTreeMap<String, (usize, u32)>)> {
    let icons_dir = config.weather_icons_dir.join("icons");
    let options = usvg::Options::default();

    let mut icon_data = Vec::new();
    let mut icon_mapping = BTreeMap::new();

    for (index, icon) in icons.iter().enumerate() {
        let svg_path = icons_dir.join(format!("{}.svg", icon.icon_code));

        if !svg_path.exists() {
            println!("cargo:warning=  SVG文件不存在: {}", svg_path.display());
            continue;
        }

        let svg_data = fs::read(&svg_path)
            .with_context(|| format!("读取SVG文件失败: {}", svg_path.display()))?;

        // 解析 SVG
        use usvg::TreeParsing;
        let tree = usvg::Tree::from_data(&svg_data, &options)
            .map_err(|e| anyhow!("解析SVG失败 {}: {}", svg_path.display(), e))?;

        // 创建像素图
        let mut pixmap =
            resvg::tiny_skia::Pixmap::new(config.weather_icon_size, config.weather_icon_size)
                .ok_or_else(|| anyhow!("创建像素图失败"))?;

        // 使用 resvg::Tree 进行渲染
        let rtree = resvg::Tree::from_usvg(&tree);

        // 计算缩放比例
        let svg_size = rtree.view_box.rect.size();
        let scale_x = config.weather_icon_size as f32 / svg_size.width();
        let scale_y = config.weather_icon_size as f32 / svg_size.height();
        let scale = scale_x.min(scale_y); // 保持宽高比

        // 计算居中偏移
        let offset_x = (config.weather_icon_size as f32 - svg_size.width() * scale) / 2.0;
        let offset_y = (config.weather_icon_size as f32 - svg_size.height() * scale) / 2.0;

        // 创建缩放和平移变换
        let transform = resvg::tiny_skia::Transform::from_scale(scale, scale)
            .post_translate(offset_x, offset_y);

        // 渲染到 pixmap
        rtree.render(transform, &mut pixmap.as_mut());

        // 转换为 1-bit 位图
        let bitmap_data = convert_to_1bit(&pixmap, config.weather_icon_size);

        // 存储图标数据
        let start_index = icon_data.len();
        icon_data.extend_from_slice(&bitmap_data);
        icon_mapping.insert(icon.icon_code.clone(), (start_index, index as u32));

        // 显示进度
        if index % 10 == 0 {
            progress.update_progress(index, icons.len(), "处理天气图标");
        }
    }

    Ok((icon_data, icon_mapping))
}

/// 将 RGBA 像素图转换为 1-bit 位图
fn convert_to_1bit(pixmap: &resvg::tiny_skia::Pixmap, icon_size: u32) -> Vec<u8> {
    let width = icon_size as usize;
    let height = icon_size as usize;

    // 计算每行所需的字节数
    let bytes_per_row = (width + 7) / 8;
    let mut result = vec![0u8; bytes_per_row * height];

    for y in 0..height {
        for x in 0..width {
            if let Some(pixel) = pixmap.pixel(x as u32, y as u32) {
                // 对于天气类图标，本身只有黑白两色，我们只需要考虑透明度和不透明度即可
                let alpha = pixel.alpha() as f32 / 255.0;

                // 阈值处理，转换为黑白
                let is_black = alpha > 0.5;

                if is_black {
                    let byte_index = y * bytes_per_row + x / 8;
                    let bit_offset = 7 - (x % 8); // MSB 优先

                    if byte_index < result.len() {
                        result[byte_index] |= 1 << bit_offset;
                    }
                }
            }
        }
    }

    result
}

/// 生成图标二进制文件
fn generate_icon_binary_files(config: &BuildConfig, icon_data: &[u8]) -> Result<()> {
    let icons_bin_path = config.output_dir.join("weather_icons.bin");
    utils::file_utils::write_file(&icons_bin_path, icon_data)?;
    Ok(())
}

/// 生成天气图标索引文件
fn generate_weather_icons_rs(config: &BuildConfig, icons: &[WeatherIcon]) -> Result<()> {
    let output_path = config.output_dir.join("weather_icons.rs");

    let content = generate_icons_rs_content(icons, config.weather_icon_size)?;
    utils::file_utils::write_string_file(&output_path, &content)?;

    println!(
        "cargo:warning=  天气图标索引文件生成成功: {}",
        output_path.display()
    );
    Ok(())
}

/// 生成天气图标Rust文件内容
fn generate_icons_rs_content(icons: &[WeatherIcon], icon_size: u32) -> Result<String> {
    let mut content = String::new();

    content.push_str("// 自动生成的天气图标数据文件\n");
    content.push_str("// 不要手动修改此文件\n\n");

    content.push_str("use embedded_graphics::{\n");
    content.push_str("    image::ImageRaw,\n");
    content.push_str("    prelude::*,\n");
    content.push_str("    pixelcolor::BinaryColor,\n");
    content.push_str("};\n\n");

    // 定义图标枚举
    content.push_str("#[allow(non_camel_case_types)]\n");
    content.push_str("#[derive(Clone, Copy, PartialEq)]\n");
    content.push_str("pub enum WeatherIcon {\n");
    for icon in icons {
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
    for i in 0..icons.len() {
        content.push_str(&format!(
            "    {}, // {}\n",
            i * bytes_per_icon,
            icons[i].icon_name
        ));
    }
    content.push_str("];\n\n");

    // 实用函数
    content.push_str("pub fn get_icon_data(icon: WeatherIcon) -> &'static [u8] {\n");
    content.push_str(&format!(
        "    let start = WEATHER_ICON_INDICES[icon.as_index()];\n"
    ));
    content.push_str(&format!("    let end = start + {};\n", bytes_per_icon));
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

    Ok(content)
}
