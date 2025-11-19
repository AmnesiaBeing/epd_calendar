//! 电池电量和充电状态图标处理模块

use crate::builder::config::BuildConfig;
use crate::builder::utils::file_utils;
use crate::builder::utils::{
    ProgressTracker,
    icon_renderer::{IconConfig, IconRenderer},
};
use anyhow::{Context, Result};
use std::collections::BTreeMap;

/// 电池电量级别和充电状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Ord, PartialOrd)]
pub enum BatteryIcon {
    Level0,   // 0% 电量
    Level1,   // 25% 电量
    Level2,   // 50% 电量
    Level3,   // 75% 电量
    Level4,   // 100% 电量
    Charging, // 充电中
}

impl BatteryIcon {
    /// 获取图标文件名
    pub fn filename(&self) -> &'static str {
        match self {
            Self::Level0 => "battery-0",
            Self::Level1 => "battery-1",
            Self::Level2 => "battery-2",
            Self::Level3 => "battery-3",
            Self::Level4 => "battery-4",
            Self::Charging => "bolt",
        }
    }

    /// 获取显示名称
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Level0 => "Battery 0%",
            Self::Level1 => "Battery 25%",
            Self::Level2 => "Battery 50%",
            Self::Level3 => "Battery 75%",
            Self::Level4 => "Battery 100%",
            Self::Charging => "Charging",
        }
    }

    /// 获取所有电池图标
    pub fn all_icons() -> Vec<Self> {
        vec![
            Self::Level0,
            Self::Level1,
            Self::Level2,
            Self::Level3,
            Self::Level4,
            Self::Charging,
        ]
    }
}

/// 构建电池图标数据
pub fn build(config: &BuildConfig, progress: &ProgressTracker) -> Result<()> {
    progress.update_progress(0, 4, "准备电池图标数据");
    let battery_icons = BatteryIcon::all_icons();

    progress.update_progress(1, 4, "渲染电池图标");
    let (icon_data, icon_mapping) = process_battery_icons(config, &battery_icons, progress)?;

    progress.update_progress(2, 4, "生成二进制文件");
    generate_battery_binary_files(config, &icon_data)?;

    progress.update_progress(3, 4, "生成索引文件");
    generate_battery_icons_rs(config, &battery_icons, &icon_mapping)?;

    println!(
        "cargo:warning=  电池图标处理完成，共处理 {} 个图标",
        icon_mapping.len()
    );
    Ok(())
}

/// 处理电池图标
fn process_battery_icons(
    _config: &BuildConfig,
    battery_icons: &[BatteryIcon],
    progress: &ProgressTracker,
) -> Result<(Vec<u8>, BTreeMap<BatteryIcon, usize>)> {
    let mut icon_data = Vec::new();
    let mut icon_mapping = BTreeMap::new();

    // 电池图标大小配置为32x32
    let battery_icon_size = 32;

    for (index, &battery_icon) in battery_icons.iter().enumerate() {
        let svg_filename = format!("{}.svg", battery_icon.filename());
        let svg_path = format!("assets/{}", svg_filename);

        // 使用通用图标渲染器渲染图标
        let icon_config = IconConfig {
            icon_size: battery_icon_size,
            svg_path: svg_path.clone(),
        };

        let result = IconRenderer::render_svg_icon(&icon_config)
            .with_context(|| format!("渲染电池图标失败: {}", battery_icon.display_name()))?;

        // 记录图标数据位置
        let start_index = icon_data.len();
        icon_mapping.insert(battery_icon, start_index);

        // 添加图标数据
        icon_data.extend_from_slice(&result.bitmap_data);

        // 显示进度
        progress.update_progress(index, battery_icons.len(), "渲染电池图标");

        println!("cargo:warning=    已处理: {}", battery_icon.display_name());
    }

    Ok((icon_data, icon_mapping))
}

/// 生成电池图标二进制文件
fn generate_battery_binary_files(config: &BuildConfig, icon_data: &[u8]) -> Result<()> {
    let battery_bin_path = config.output_dir.join("battery_icons.bin");
    file_utils::write_file(&battery_bin_path, icon_data)?;

    println!(
        "cargo:warning=  电池图标二进制文件生成成功: {}",
        battery_bin_path.display()
    );

    Ok(())
}

/// 生成电池图标索引文件
fn generate_battery_icons_rs(
    config: &BuildConfig,
    battery_icons: &[BatteryIcon],
    icon_mapping: &BTreeMap<BatteryIcon, usize>,
) -> Result<()> {
    let output_path = config.output_dir.join("battery_icons.rs");

    let content = generate_battery_icons_rs_content(battery_icons, icon_mapping)?;
    file_utils::write_string_file(&output_path, &content)?;

    println!(
        "cargo:warning=  电池图标索引文件生成成功: {}",
        output_path.display()
    );
    Ok(())
}

/// 生成电池图标Rust文件内容
fn generate_battery_icons_rs_content(
    battery_icons: &[BatteryIcon],
    icon_mapping: &BTreeMap<BatteryIcon, usize>,
) -> Result<String> {
    let mut content = String::new();

    content.push_str("// 自动生成的电池图标数据文件\n");
    content.push_str("// 不要手动修改此文件\n\n");

    content.push_str("use embedded_graphics::{\n");
    content.push_str("    image::ImageRaw,\n");
    content.push_str("    prelude::*,\n");
    content.push_str("    pixelcolor::BinaryColor,\n");
    content.push_str("};\n\n");

    // 定义电池图标枚举
    content.push_str("#[derive(Clone, Copy, PartialEq, Debug)]\n");
    content.push_str("pub enum BatteryIcon {\n");
    for battery_icon in battery_icons {
        let variant_name = match battery_icon {
            BatteryIcon::Level0 => "Level0",
            BatteryIcon::Level1 => "Level1",
            BatteryIcon::Level2 => "Level2",
            BatteryIcon::Level3 => "Level3",
            BatteryIcon::Level4 => "Level4",
            BatteryIcon::Charging => "Charging",
        };
        content.push_str(&format!(
            "    {}, // {}\n",
            variant_name,
            battery_icon.display_name()
        ));
    }
    content.push_str("}\n\n");

    // 实现枚举方法
    content.push_str("impl BatteryIcon {\n");

    // as_index 方法
    content.push_str("    pub fn as_index(&self) -> usize {\n");
    content.push_str("        match self {\n");
    for (index, battery_icon) in battery_icons.iter().enumerate() {
        let variant_name = match battery_icon {
            BatteryIcon::Level0 => "Level0",
            BatteryIcon::Level1 => "Level1",
            BatteryIcon::Level2 => "Level2",
            BatteryIcon::Level3 => "Level3",
            BatteryIcon::Level4 => "Level4",
            BatteryIcon::Charging => "Charging",
        };
        content.push_str(&format!(
            "            BatteryIcon::{} => {},\n",
            variant_name, index
        ));
    }
    content.push_str("        }\n");
    content.push_str("    }\n\n");

    // from_level 方法 - 根据电量百分比获取对应图标
    content.push_str("    pub fn from_level(level: u8) -> Self {\n");
    content.push_str("        match level {\n");
    content.push_str("            0..=10 => BatteryIcon::Level0,\n");
    content.push_str("            11..=35 => BatteryIcon::Level1,\n");
    content.push_str("            36..=60 => BatteryIcon::Level2,\n");
    content.push_str("            61..=85 => BatteryIcon::Level3,\n");
    content.push_str("            86..=100 => BatteryIcon::Level4,\n");
    content.push_str("            _ => BatteryIcon::Level0,\n");
    content.push_str("        }\n");
    content.push_str("    }\n\n");

    // display_name 方法
    content.push_str("    pub fn display_name(&self) -> &'static str {\n");
    content.push_str("        match self {\n");
    for battery_icon in battery_icons {
        let variant_name = match battery_icon {
            BatteryIcon::Level0 => "Level0",
            BatteryIcon::Level1 => "Level1",
            BatteryIcon::Level2 => "Level2",
            BatteryIcon::Level3 => "Level3",
            BatteryIcon::Level4 => "Level4",
            BatteryIcon::Charging => "Charging",
        };
        content.push_str(&format!(
            "            BatteryIcon::{} => \"{}\",\n",
            variant_name,
            battery_icon.display_name()
        ));
    }
    content.push_str("        }\n");
    content.push_str("    }\n");
    content.push_str("}\n\n");

    // 定义图标数据
    content.push_str(
        "pub const BATTERY_ICON_DATA: &[u8] = include_bytes!(\"battery_icons.bin\");\n\n",
    );
    content.push_str("pub const BATTERY_ICON_SIZE: u32 = 32;\n");
    content.push_str(&format!(
        "pub const BATTERY_ICON_COUNT: usize = {};\n\n",
        battery_icons.len()
    ));

    // 生成图标索引数组
    content.push_str("pub const BATTERY_ICON_INDICES: &[usize] = &[\n");
    for battery_icon in battery_icons {
        if let Some(&start_index) = icon_mapping.get(battery_icon) {
            content.push_str(&format!(
                "    {}, // {}\n",
                start_index,
                battery_icon.display_name()
            ));
        }
    }
    content.push_str("];\n\n");

    // 计算每个图标的字节大小 (32x32 / 8 = 128 bytes)
    let bytes_per_icon = 128;

    // 实用函数
    content.push_str("pub fn get_battery_icon_data(icon: BatteryIcon) -> &'static [u8] {\n");
    content.push_str("    let start = BATTERY_ICON_INDICES[icon.as_index()];\n");
    content.push_str(&format!("    let end = start + {};\n", bytes_per_icon));
    content.push_str("    &BATTERY_ICON_DATA[start..end]\n");
    content.push_str("}\n\n");

    // 添加便捷函数来创建 ImageRaw
    content.push_str("pub fn get_battery_icon_image_raw(icon: BatteryIcon) -> ImageRaw<'static, BinaryColor> {\n");
    content.push_str("    ImageRaw::new(get_battery_icon_data(icon), 32)\n");
    content.push_str("}\n\n");

    // 添加便捷函数获取所有可用图标
    content.push_str("pub fn get_all_battery_icons() -> &'static [BatteryIcon] {\n");
    content.push_str("    &[\n");
    for battery_icon in battery_icons {
        let variant_name = match battery_icon {
            BatteryIcon::Level0 => "Level0",
            BatteryIcon::Level1 => "Level1",
            BatteryIcon::Level2 => "Level2",
            BatteryIcon::Level3 => "Level3",
            BatteryIcon::Level4 => "Level4",
            BatteryIcon::Charging => "Charging",
        };
        content.push_str(&format!("        BatteryIcon::{},\n", variant_name));
    }
    content.push_str("    ]\n");
    content.push_str("}\n\n");

    // 添加便捷函数根据电量和充电状态获取图标
    content.push_str("pub fn get_battery_icon(level: u8, is_charging: bool) -> BatteryIcon {\n");
    content.push_str("    if is_charging {\n");
    content.push_str("        BatteryIcon::Charging\n");
    content.push_str("    } else {\n");
    content.push_str("        BatteryIcon::from_level(level)\n");
    content.push_str("    }\n");
    content.push_str("}\n");

    Ok(content)
}
