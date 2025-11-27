//! 电池电量和充电状态图标处理模块

use crate::builder::config::BuildConfig;
use crate::builder::utils::file_utils;
use crate::builder::utils::{
    icon_renderer::{IconConfig, IconRenderer},
    progress::ProgressTracker,
};
use anyhow::{Context, Result};
use std::collections::BTreeMap;

const DIGIT_ICON_WIDTH: u32 = 72;
const DIGIT_ICON_HEIGHT: u32 = 128;

/// 数字图标
#[derive(Debug, Clone, Copy, PartialEq, Eq, Ord, PartialOrd)]
pub enum DigitIcon {
    Zero,
    One,
    Two,
    Three,
    Four,
    Five,
    Six,
    Seven,
    Eight,
    Nine,
    Seperator,
    Colon,
}

impl DigitIcon {
    /// 获取图标文件名
    pub fn filename(&self) -> &'static str {
        match self {
            Self::Zero => "digit_0",
            Self::One => "digit_1",
            Self::Two => "digit_2",
            Self::Three => "digit_3",
            Self::Four => "digit_4",
            Self::Five => "digit_5",
            Self::Six => "digit_6",
            Self::Seven => "digit_7",
            Self::Eight => "digit_8",
            Self::Nine => "digit_9",
            Self::Seperator => "digit_sep",
            Self::Colon => "digit_colon",
        }
    }

    pub fn all_icons() -> Vec<Self> {
        vec![
            Self::Seperator,
            Self::Colon,
            Self::Zero,
            Self::One,
            Self::Two,
            Self::Three,
            Self::Four,
            Self::Five,
            Self::Six,
            Self::Seven,
            Self::Eight,
            Self::Nine,
        ]
    }
}

/// 构建数字图标数据
pub fn build(config: &BuildConfig, progress: &ProgressTracker) -> Result<()> {
    progress.update_progress(0, 4, "准备数字图标数据");
    let digit_icons = DigitIcon::all_icons();

    progress.update_progress(1, 4, "渲染数字图标");
    let (icon_data, icon_mapping) = process_digit_icons(config, &digit_icons, progress)?;

    progress.update_progress(2, 4, "生成二进制文件");
    generate_digit_binary_files(config, &icon_data)?;

    progress.update_progress(3, 4, "生成索引文件");
    generate_digit_icons_rs(config, &digit_icons, &icon_mapping)?;

    println!(
        "cargo:warning=  数字图标处理完成，共处理 {} 个图标",
        icon_mapping.len()
    );
    Ok(())
}

/// 处理数字图标
fn process_digit_icons(
    _config: &BuildConfig,
    digit_icons: &[DigitIcon],
    progress: &ProgressTracker,
) -> Result<(Vec<u8>, BTreeMap<DigitIcon, usize>)> {
    let mut icon_data = Vec::new();
    let mut icon_mapping = BTreeMap::new();
    let mut preview_results = Vec::new(); // 用于存储预览数据

    for (index, &digit_icon) in digit_icons.iter().enumerate() {
        let svg_filename = format!("{}.svg", digit_icon.filename());
        let svg_path = format!("assets/{}", svg_filename);

        // 使用通用图标渲染器渲染图标
        let icon_config = IconConfig {
            target_width: DIGIT_ICON_WIDTH,
            target_height: DIGIT_ICON_HEIGHT,
            svg_path: svg_path.clone(),
        };

        let result = IconRenderer::render_svg_icon(&icon_config)
            .with_context(|| format!("渲染数字图标失败: {:?}", digit_icon))?;

        // 记录图标数据位置
        let start_index = icon_data.len();
        icon_mapping.insert(digit_icon, start_index);

        // 添加图标数据
        icon_data.extend_from_slice(&result.bitmap_data);

        // 存储预览数据
        preview_results.push((icon_config, result));

        // 显示进度
        progress.update_progress(index, digit_icons.len(), "渲染数字图标");

        println!("cargo:warning=    已处理: {:?}", digit_icon);
    }

    // 预览其中一个图标
    let (icon_config, result) = preview_results.last().unwrap();
    IconRenderer::preview_icon(result, icon_config);

    Ok((icon_data, icon_mapping))
}

/// 生成数字图标二进制文件
fn generate_digit_binary_files(config: &BuildConfig, icon_data: &[u8]) -> Result<()> {
    let digit_bin_path = config.output_dir.join("generated_digit_icons.bin");
    file_utils::write_file(&digit_bin_path, icon_data)?;

    println!(
        "cargo:warning=  数字图标二进制文件生成成功: {}",
        digit_bin_path.display()
    );

    Ok(())
}

/// 生成数字图标索引文件
fn generate_digit_icons_rs(
    config: &BuildConfig,
    digit_icons: &[DigitIcon],
    icon_mapping: &BTreeMap<DigitIcon, usize>,
) -> Result<()> {
    let output_path = config.output_dir.join("generated_digit_icons.rs");

    let content = generate_digit_icons_rs_content(digit_icons, icon_mapping)?;
    file_utils::write_string_file(&output_path, &content)?;

    println!(
        "cargo:warning=  数字图标索引文件生成成功: {}",
        output_path.display()
    );
    Ok(())
}

/// 生成数字图标Rust文件内容
fn generate_digit_icons_rs_content(
    digit_icons: &[DigitIcon],
    icon_mapping: &BTreeMap<DigitIcon, usize>,
) -> Result<String> {
    let mut content = String::new();

    content.push_str("// 自动生成的电池图标数据文件\n");
    content.push_str("// 不要手动修改此文件\n\n");

    // 定义数字图标枚举
    content.push_str("#[derive(Clone, Copy, PartialEq, Debug)]\n");
    content.push_str("pub enum DigitIcon {\n");
    for digit_icon in digit_icons {
        let variant_name = match digit_icon {
            DigitIcon::Zero => "Zero",
            DigitIcon::One => "One",
            DigitIcon::Two => "Two",
            DigitIcon::Three => "Three",
            DigitIcon::Four => "Four",
            DigitIcon::Five => "Five",
            DigitIcon::Six => "Six",
            DigitIcon::Seven => "Seven",
            DigitIcon::Eight => "Eight",
            DigitIcon::Nine => "Nine",
            DigitIcon::Seperator => "Seperator",
            DigitIcon::Colon => "Colon",
        };
        content.push_str(&format!("    {}, // {:?}\n", variant_name, digit_icon));
    }
    content.push_str("}\n\n");

    // 实现枚举方法
    content.push_str("impl DigitIcon {\n");

    // as_index 方法
    content.push_str("    pub fn as_index(&self) -> usize {\n");
    content.push_str("        match self {\n");
    for (index, digit_icon) in digit_icons.iter().enumerate() {
        let variant_name = match digit_icon {
            DigitIcon::Zero => "Zero",
            DigitIcon::One => "One",
            DigitIcon::Two => "Two",
            DigitIcon::Three => "Three",
            DigitIcon::Four => "Four",
            DigitIcon::Five => "Five",
            DigitIcon::Six => "Six",
            DigitIcon::Seven => "Seven",
            DigitIcon::Eight => "Eight",
            DigitIcon::Nine => "Nine",
            DigitIcon::Seperator => "Seperator",
            DigitIcon::Colon => "Colon",
        };
        content.push_str(&format!(
            "            DigitIcon::{} => {},\n",
            variant_name, index
        ));
    }
    content.push_str("        }\n");
    content.push_str("    }\n\n");
    content.push_str("}\n\n");

    // 定义图标数据
    content.push_str(
        "pub const DIGIT_ICON_DATA: &[u8] = include_bytes!(\"generated_digit_icons.bin\");\n\n",
    );
    content.push_str(&format!(
        "pub const DIGIT_ICON_WIDTH: u32 = {};\n\n",
        DIGIT_ICON_WIDTH
    ));
    content.push_str(&format!(
        "pub const DIGIT_ICON_HEIGHT: u32 = {};\n\n",
        DIGIT_ICON_HEIGHT
    ));

    // 生成图标索引数组
    content.push_str("pub const DIGIT_ICON_INDICES: &[usize] = &[\n");
    for digit_icon in digit_icons {
        if let Some(&start_index) = icon_mapping.get(digit_icon) {
            content.push_str(&format!("    {}, // {:?}\n", start_index, digit_icon));
        }
    }
    content.push_str("];\n\n");

    // 计算每个图标的字节大小
    let bytes_per_icon = DIGIT_ICON_WIDTH * DIGIT_ICON_HEIGHT / 8;

    // 实用函数
    content.push_str("pub fn get_digit_icon_data(icon: DigitIcon) -> &'static [u8] {\n");
    content.push_str("    let start = DIGIT_ICON_INDICES[icon.as_index()];\n");
    content.push_str(&format!("    let end = start + {};\n", bytes_per_icon));
    content.push_str("    &DIGIT_ICON_DATA[start..end]\n");
    content.push_str("}\n\n");

    Ok(content)
}
