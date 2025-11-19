//! 网络连接状态图标处理模块

use crate::builder::config::BuildConfig;
use crate::builder::utils::file_utils;
use crate::builder::utils::{
    ProgressTracker,
    icon_renderer::{IconConfig, IconRenderer},
};
use anyhow::{Context, Result};
use std::collections::BTreeMap;

/// 网络连接状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Ord, PartialOrd, Hash)]
pub enum NetworkIcon {
    Connected,    // 已连接网络
    Disconnected, // 未连接网络
}

impl NetworkIcon {
    /// 获取图标文件名
    pub fn filename(&self) -> &'static str {
        match self {
            Self::Connected => "connected",
            Self::Disconnected => "unconnected",
        }
    }

    /// 获取显示名称
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Connected => "Network Connected",
            Self::Disconnected => "Network Disconnected",
        }
    }

    /// 获取所有网络图标
    pub fn all_icons() -> Vec<Self> {
        vec![Self::Connected, Self::Disconnected]
    }

    /// 根据连接状态获取图标
    pub fn from_status(is_connected: bool) -> Self {
        if is_connected {
            Self::Connected
        } else {
            Self::Disconnected
        }
    }
}

/// 构建网络图标数据
pub fn build(config: &BuildConfig, progress: &ProgressTracker) -> Result<()> {
    progress.update_progress(0, 4, "准备网络图标数据");
    let network_icons = NetworkIcon::all_icons();

    progress.update_progress(1, 4, "渲染网络图标");
    let (icon_data, icon_mapping) = process_network_icons(config, &network_icons, progress)?;

    progress.update_progress(2, 4, "生成二进制文件");
    generate_network_binary_files(config, &icon_data)?;

    progress.update_progress(3, 4, "生成索引文件");
    generate_network_icons_rs(config, &network_icons, &icon_mapping)?;

    println!(
        "cargo:warning=  网络图标处理完成，共处理 {} 个图标",
        icon_mapping.len()
    );
    Ok(())
}

/// 处理网络图标
fn process_network_icons(
    config: &BuildConfig,
    network_icons: &[NetworkIcon],
    progress: &ProgressTracker,
) -> Result<(Vec<u8>, BTreeMap<NetworkIcon, usize>)> {
    let mut icon_data = Vec::new();
    let mut icon_mapping = BTreeMap::new();

    // 网络图标大小配置为24x24（适合状态栏显示）
    let network_icon_size = 24;

    for (index, &network_icon) in network_icons.iter().enumerate() {
        let svg_filename = format!("{}.svg", network_icon.filename());
        let svg_path = format!("assets/{}", svg_filename);

        // 使用通用图标渲染器渲染图标
        let icon_config = IconConfig {
            icon_size: network_icon_size,
            svg_path: svg_path.clone(),
        };

        let result = IconRenderer::render_svg_icon(&icon_config)
            .with_context(|| format!("渲染网络图标失败: {}", network_icon.display_name()))?;

        // 记录图标数据位置
        let start_index = icon_data.len();
        icon_mapping.insert(network_icon, start_index);

        // 添加图标数据
        icon_data.extend_from_slice(&result.bitmap_data);

        // 显示进度
        progress.update_progress(index, network_icons.len(), "渲染网络图标");

        println!("cargo:warning=    已处理: {}", network_icon.display_name());
    }

    Ok((icon_data, icon_mapping))
}

/// 生成网络图标二进制文件
fn generate_network_binary_files(config: &BuildConfig, icon_data: &[u8]) -> Result<()> {
    let network_bin_path = config.output_dir.join("network_icons.bin");
    file_utils::write_file(&network_bin_path, icon_data)?;

    println!(
        "cargo:warning=  网络图标二进制文件生成成功: {}",
        network_bin_path.display()
    );

    Ok(())
}

/// 生成网络图标索引文件
fn generate_network_icons_rs(
    config: &BuildConfig,
    network_icons: &[NetworkIcon],
    icon_mapping: &BTreeMap<NetworkIcon, usize>,
) -> Result<()> {
    let output_path = config.output_dir.join("network_icons.rs");

    let content = generate_network_icons_rs_content(network_icons, icon_mapping)?;
    file_utils::write_string_file(&output_path, &content)?;

    println!(
        "cargo:warning=  网络图标索引文件生成成功: {}",
        output_path.display()
    );
    Ok(())
}

/// 生成网络图标Rust文件内容
fn generate_network_icons_rs_content(
    network_icons: &[NetworkIcon],
    icon_mapping: &BTreeMap<NetworkIcon, usize>,
) -> Result<String> {
    let mut content = String::new();

    content.push_str("// 自动生成的网络连接状态图标数据文件\n");
    content.push_str("// 不要手动修改此文件\n\n");

    content.push_str("use embedded_graphics::{\n");
    content.push_str("    image::ImageRaw,\n");
    content.push_str("    prelude::*,\n");
    content.push_str("    pixelcolor::BinaryColor,\n");
    content.push_str("};\n\n");

    // 定义网络图标枚举
    content.push_str("#[derive(Clone, Copy, PartialEq, Debug)]\n");
    content.push_str("pub enum NetworkIcon {\n");
    for network_icon in network_icons {
        let variant_name = match network_icon {
            NetworkIcon::Connected => "Connected",
            NetworkIcon::Disconnected => "Disconnected",
        };
        content.push_str(&format!(
            "    {}, // {}\n",
            variant_name,
            network_icon.display_name()
        ));
    }
    content.push_str("}\n\n");

    // 实现枚举方法
    content.push_str("impl NetworkIcon {\n");

    // as_index 方法
    content.push_str("    pub fn as_index(&self) -> usize {\n");
    content.push_str("        match self {\n");
    for (index, network_icon) in network_icons.iter().enumerate() {
        let variant_name = match network_icon {
            NetworkIcon::Connected => "Connected",
            NetworkIcon::Disconnected => "Disconnected",
        };
        content.push_str(&format!(
            "            NetworkIcon::{} => {},\n",
            variant_name, index
        ));
    }
    content.push_str("        }\n");
    content.push_str("    }\n\n");

    // from_status 方法 - 根据连接状态获取对应图标
    content.push_str("    pub fn from_status(is_connected: bool) -> Self {\n");
    content.push_str("        if is_connected {\n");
    content.push_str("            NetworkIcon::Connected\n");
    content.push_str("        } else {\n");
    content.push_str("            NetworkIcon::Disconnected\n");
    content.push_str("        }\n");
    content.push_str("    }\n\n");

    // display_name 方法
    content.push_str("    pub fn display_name(&self) -> &'static str {\n");
    content.push_str("        match self {\n");
    for network_icon in network_icons {
        let variant_name = match network_icon {
            NetworkIcon::Connected => "Connected",
            NetworkIcon::Disconnected => "Disconnected",
        };
        content.push_str(&format!(
            "            NetworkIcon::{} => \"{}\",\n",
            variant_name,
            network_icon.display_name()
        ));
    }
    content.push_str("        }\n");
    content.push_str("    }\n\n");

    // is_connected 方法 - 检查当前图标是否表示已连接状态
    content.push_str("    pub fn is_connected(&self) -> bool {\n");
    content.push_str("        match self {\n");
    content.push_str("            NetworkIcon::Connected => true,\n");
    content.push_str("            NetworkIcon::Disconnected => false,\n");
    content.push_str("        }\n");
    content.push_str("    }\n");
    content.push_str("}\n\n");

    // 定义图标数据
    content.push_str(
        "pub const NETWORK_ICON_DATA: &[u8] = include_bytes!(\"network_icons.bin\");\n\n",
    );
    content.push_str("pub const NETWORK_ICON_SIZE: u32 = 24;\n");
    content.push_str(&format!(
        "pub const NETWORK_ICON_COUNT: usize = {};\n\n",
        network_icons.len()
    ));

    // 生成图标索引数组
    content.push_str("pub const NETWORK_ICON_INDICES: &[usize] = &[\n");
    for network_icon in network_icons {
        if let Some(&start_index) = icon_mapping.get(network_icon) {
            content.push_str(&format!(
                "    {}, // {}\n",
                start_index,
                network_icon.display_name()
            ));
        }
    }
    content.push_str("];\n\n");

    // 计算每个图标的字节大小 (24x24 / 8 = 72 bytes)
    let bytes_per_icon = 72;

    // 实用函数
    content.push_str("pub fn get_network_icon_data(icon: NetworkIcon) -> &'static [u8] {\n");
    content.push_str("    let start = NETWORK_ICON_INDICES[icon.as_index()];\n");
    content.push_str(&format!("    let end = start + {};\n", bytes_per_icon));
    content.push_str("    &NETWORK_ICON_DATA[start..end]\n");
    content.push_str("}\n\n");

    // 添加便捷函数来创建 ImageRaw
    content.push_str("pub fn get_network_icon_image_raw(icon: NetworkIcon) -> ImageRaw<'static, BinaryColor> {\n");
    content.push_str("    ImageRaw::new(get_network_icon_data(icon), 24)\n");
    content.push_str("}\n\n");

    // 添加便捷函数获取所有可用图标
    content.push_str("pub fn get_all_network_icons() -> &'static [NetworkIcon] {\n");
    content.push_str("    &[\n");
    for network_icon in network_icons {
        let variant_name = match network_icon {
            NetworkIcon::Connected => "Connected",
            NetworkIcon::Disconnected => "Disconnected",
        };
        content.push_str(&format!("        NetworkIcon::{},\n", variant_name));
    }
    content.push_str("    ]\n");
    content.push_str("}\n\n");

    // 添加便捷函数根据连接状态获取图标
    content.push_str("pub fn get_network_icon(is_connected: bool) -> NetworkIcon {\n");
    content.push_str("    NetworkIcon::from_status(is_connected)\n");
    content.push_str("}\n\n");

    // 为 Option<bool> 提供便捷函数
    content.push_str(
        "pub fn get_network_icon_optional(connection_status: Option<bool>) -> NetworkIcon {\n",
    );
    content.push_str("    match connection_status {\n");
    content.push_str("        Some(true) => NetworkIcon::Connected,\n");
    content.push_str("        Some(false) => NetworkIcon::Disconnected,\n");
    content.push_str("        None => NetworkIcon::Disconnected, // 默认显示为未连接\n");
    content.push_str("    }\n");
    content.push_str("}\n");

    Ok(content)
}
