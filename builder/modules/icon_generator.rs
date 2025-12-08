//! 图标生成模块
//! 整合本地静态图标与天气图标生成逻辑，输出统一的枚举、位图数据及辅助方法

use anyhow::{Context, Result, bail};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::builder::config::{BuildConfig, LocalIconCategoryConfig, WeatherIconConfig};
use crate::builder::utils::file_utils;
use crate::builder::utils::icon_renderer::{IconConfig, IconRenderResult, IconRenderer};
use crate::builder::utils::progress::ProgressTracker;

/// 图标的尺寸信息
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IconSize {
    pub width: u32,
    pub height: u32,
}

impl IconSize {
    /// 计算位图数据长度（单色位图，每像素1位）
    pub fn bitmap_len(&self) -> usize {
        ((self.width * self.height + 7) / 8) as usize
    }

    /// 创建空白位图数据
    pub fn create_blank_bitmap(&self) -> Vec<u8> {
        vec![0; self.bitmap_len()]
    }
}

/// 处理后的图标信息（统一的Trait对象）
#[derive(Debug, Clone)]
struct ProcessedIconInfo {
    pub id: String,
    pub variant_name: String,
    pub bitmap_data: Vec<u8>,
    pub size: IconSize,
    pub source_type: IconSourceType,
}

/// 图标来源类型
#[derive(Debug, Clone, PartialEq, Eq)]
enum IconSourceType {
    Local { category: String, filename: String },
    Weather { icon_code: String },
}

/// 图标处理统计信息
#[derive(Debug, Default)]
struct ProcessingStats {
    total_icons: usize,
    skipped_icons: usize,
    error_icons: usize,
}

impl ProcessingStats {
    fn record_success(&mut self) {
        self.total_icons += 1;
    }

    fn record_skipped(&mut self) {
        self.skipped_icons += 1;
        self.total_icons += 1;
    }

    fn record_error(&mut self) {
        self.error_icons += 1;
        self.total_icons += 1;
    }
}

/// 构建所有图标数据
pub fn build(config: &BuildConfig, progress: &ProgressTracker) -> Result<()> {
    progress.update_progress(0, 4, "准备构建图标");

    // 1. 处理本地静态图标
    progress.update_progress(1, 4, "处理本地静态图标");
    let local_icons = process_local_icon_categories(config, progress)?;

    // 2. 处理天气图标
    progress.update_progress(2, 4, "处理天气图标");
    let weather_icons = process_weather_icons(config, progress)?;

    // 3. 合并所有图标并进行唯一性校验
    progress.update_progress(3, 4, "合并和校验图标");
    let all_icons = merge_and_validate_icons(&local_icons, &weather_icons)?;

    // 4. 生成统一的图标文件
    progress.update_progress(4, 4, "生成统一图标文件");
    generate_unified_icon_file(config, &all_icons)?;

    println!("cargo:warning=  所有图标处理完成");
    Ok(())
}

/// 处理本地静态图标分类
fn process_local_icon_categories(
    config: &BuildConfig,
    progress: &ProgressTracker,
) -> Result<Vec<ProcessedIconInfo>> {
    let mut stats = ProcessingStats::default();
    let total_categories = config.local_icon_categories.len();
    let mut all_icons = Vec::new();

    for (cat_idx, category) in config.local_icon_categories.iter().enumerate() {
        progress.update_progress(
            cat_idx,
            total_categories,
            &format!("处理{}图标", category.category),
        );

        match process_local_icon_category(category, &mut stats) {
            Ok(icons) => all_icons.extend(icons),
            Err(e) => {
                println!(
                    "cargo:warning=Error: 处理{}图标分类失败: {}",
                    category.category, e,
                );
            }
        }
    }

    if stats.error_icons > 0 {
        println!(
            "cargo:warning=Warning: 本地图标处理中有{}个错误",
            stats.error_icons,
        );
    }

    Ok(all_icons)
}

/// 处理单个本地静态图标分类
fn process_local_icon_category(
    category: &LocalIconCategoryConfig,
    stats: &mut ProcessingStats,
) -> Result<Vec<ProcessedIconInfo>> {
    // 检查目录是否存在
    if !category.dir.exists() {
        bail!("图标目录不存在: {:?}", category.dir);
    }

    // 收集SVG文件
    let svg_files = collect_svg_files(&category.dir)?;
    if svg_files.is_empty() {
        bail!("目录中没有找到SVG文件: {:?}", category.dir);
    }

    // 顺序处理图标
    let mut processed_icons = Vec::new();

    for svg_path in &svg_files {
        match process_single_local_icon(svg_path, category, stats) {
            Ok(icon) => processed_icons.push(icon),
            Err(e) => {
                stats.record_error();
                println!("cargo:warning=Error: 处理图标失败: {}", e);
                // 继续处理其他图标而不是立即失败
            }
        }
    }

    // 检查是否至少有一个成功处理的图标
    if processed_icons.is_empty() {
        bail!("没有成功处理的图标");
    }

    // 校验尺寸一致性
    validate_icon_sizes(&processed_icons, &category.category)?;

    Ok(processed_icons)
}

/// 收集目录中的所有SVG文件
fn collect_svg_files(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut svg_files = Vec::new();

    for entry in fs::read_dir(dir).with_context(|| format!("读取目录失败: {:?}", dir))? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() && path.extension().map_or(false, |ext| ext == "svg") {
            svg_files.push(path);
        }
    }

    svg_files.sort(); // 确保顺序一致性
    Ok(svg_files)
}

/// 处理单个本地图标
fn process_single_local_icon(
    svg_path: &Path,
    category: &LocalIconCategoryConfig,
    stats: &mut ProcessingStats,
) -> Result<ProcessedIconInfo> {
    // 获取文件名（不含扩展名）
    let filename = svg_path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| anyhow::anyhow!("获取文件名失败: {:?}", svg_path))?
        .to_string();

    // 转换为Rust枚举变体名
    let variant_name = convert_to_variant_name(&filename);

    // 渲染图标
    let render_result = render_svg(svg_path, category.width, category.height)?;

    let size = IconSize {
        width: category.width,
        height: category.height,
    };

    stats.record_success();

    Ok(ProcessedIconInfo {
        id: filename.clone(),
        variant_name,
        bitmap_data: render_result.bitmap_data,
        size,
        source_type: IconSourceType::Local {
            category: category.category.clone(),
            filename,
        },
    })
}

/// 处理天气图标
fn process_weather_icons(
    config: &BuildConfig,
    progress: &ProgressTracker,
) -> Result<Vec<ProcessedIconInfo>> {
    let weather_config = &config.weather_icon_config;
    let mut stats = ProcessingStats::default();

    // 加载并处理图标列表
    let icons = load_and_process_weather_icons(weather_config, &mut stats, progress)?;

    if stats.error_icons > 0 {
        println!(
            "cargo:warning=Warning: 天气图标处理中有{}个错误",
            stats.error_icons,
        );
    }

    Ok(icons)
}

/// 加载并处理天气图标列表
fn load_and_process_weather_icons(
    config: &WeatherIconConfig,
    stats: &mut ProcessingStats,
    progress: &ProgressTracker,
) -> Result<Vec<ProcessedIconInfo>> {
    // 加载图标列表
    let icons_list = load_weather_icon_list(config)?;

    // 过滤-fill图标
    let filtered_icons = filter_fill_weather_icons(&icons_list);

    if filtered_icons.is_empty() {
        bail!("过滤后没有天气图标");
    }

    // 顺序处理天气图标
    let total_icons = filtered_icons.len();
    let mut weather_icons = Vec::new();

    for (idx, icon) in filtered_icons.iter().enumerate() {
        // 每100个图标打印一次进度，或者处理第一个图标时打印一次
        if idx == 0 || idx % 100 == 0 {
            progress.update_progress(idx, total_icons, "处理天气图标");
        }

        match process_single_weather_icon(icon, config, stats) {
            Ok(processed_icon) => weather_icons.push(processed_icon),
            Err(e) => {
                stats.record_error();
                println!("cargo:warning=Error: 处理天气图标失败: {}", e);
            }
        }
    }

    // 校验尺寸一致性
    if !weather_icons.is_empty() {
        validate_icon_sizes(&weather_icons, "天气图标")?;
    }

    Ok(weather_icons)
}

/// 天气图标JSON结构
#[derive(Debug, serde::Deserialize, Clone)]
struct WeatherIconListEntry {
    icon_code: String,
    icon_name: String,
}

/// 加载天气图标列表
fn load_weather_icon_list(config: &WeatherIconConfig) -> Result<Vec<WeatherIconListEntry>> {
    let content = fs::read_to_string(&config.list_path)
        .with_context(|| format!("读取天气图标列表失败: {:?}", config.list_path))?;

    let icons: Vec<WeatherIconListEntry> =
        serde_json::from_str(&content).context("解析天气图标列表JSON失败")?;

    if icons.is_empty() {
        bail!("天气图标列表为空");
    }

    Ok(icons)
}

/// 过滤掉-fill天气图标，保留非fill版本
fn filter_fill_weather_icons(icons: &[WeatherIconListEntry]) -> Vec<WeatherIconListEntry> {
    let mut icon_map = HashMap::new();

    for icon in icons {
        if icon.icon_code.ends_with("-fill") {
            continue;
        }

        let fill_code = format!("{}-fill", icon.icon_code);
        let has_fill_version = icons.iter().any(|i| i.icon_code == fill_code);

        if has_fill_version {
            // 检查是否已经有对应的非fill版本
            icon_map
                .entry(icon.icon_code.clone())
                .or_insert_with(|| icon.clone());
        } else {
            icon_map
                .entry(icon.icon_code.clone())
                .or_insert_with(|| icon.clone());
        }
    }

    let mut filtered: Vec<_> = icon_map.into_values().collect();
    filtered.sort_by(|a, b| a.icon_code.cmp(&b.icon_code));
    filtered
}

/// 处理单个天气图标
fn process_single_weather_icon(
    icon: &WeatherIconListEntry,
    config: &WeatherIconConfig,
    stats: &mut ProcessingStats,
) -> Result<ProcessedIconInfo> {
    let svg_path = config
        .dir
        .join("icons")
        .join(format!("{}.svg", icon.icon_code));

    let (bitmap_data, size) = if svg_path.exists() {
        let render_result = render_svg(&svg_path, config.width, config.height)?;
        let size = IconSize {
            width: config.width,
            height: config.height,
        };
        (render_result.bitmap_data, size)
    } else {
        eprintln!(
            "Warning: 天气图标文件不存在，使用空白图标: {}",
            icon.icon_code
        );
        stats.record_skipped();

        let size = IconSize {
            width: config.width,
            height: config.height,
        };
        (size.create_blank_bitmap(), size)
    };

    let variant_name = if icon.icon_code.chars().all(|c| c.is_ascii_digit()) {
        format!("Icon{}", icon.icon_code)
    } else {
        convert_to_variant_name(&icon.icon_name)
    };

    stats.record_success();

    Ok(ProcessedIconInfo {
        id: icon.icon_code.clone(),
        variant_name,
        bitmap_data,
        size,
        source_type: IconSourceType::Weather {
            icon_code: icon.icon_code.clone(),
        },
    })
}

/// 渲染SVG图标（通用函数）
fn render_svg(svg_path: &Path, width: u32, height: u32) -> Result<IconRenderResult> {
    let svg_path_str = svg_path.to_string_lossy().to_string();
    let icon_config = IconConfig {
        target_width: width,
        target_height: height,
        svg_path: svg_path_str,
    };

    IconRenderer::render_svg_icon(&icon_config)
        .with_context(|| format!("渲染SVG图标失败: {:?}", svg_path))
}

/// 转换文件名到Rust枚举变体名
fn convert_to_variant_name(filename: &str) -> String {
    // 移除常见的前缀和特殊字符
    let cleaned = filename
        .trim_start_matches("icon-")
        .trim_start_matches("ic_")
        .trim_start_matches("ui-");

    let mut result = String::new();
    let mut capitalize_next = true;
    let mut last_char_was_upper = false;

    for c in cleaned.chars() {
        match c {
            '-' | '_' | '.' | ' ' => {
                capitalize_next = true;
            }
            c if c.is_ascii_alphanumeric() => {
                if capitalize_next {
                    result.push(c.to_ascii_uppercase());
                    capitalize_next = false;
                    last_char_was_upper = true;
                } else if last_char_was_upper && c.is_ascii_uppercase() {
                    // 处理连续大写字母的情况（如USA）
                    result.push(c);
                } else {
                    result.push(c.to_ascii_lowercase());
                    last_char_was_upper = false;
                }
            }
            _ => {
                // 跳过其他字符
            }
        }
    }

    // 确保以字母开头
    if let Some(first_char) = result.chars().next() {
        if !first_char.is_ascii_alphabetic() {
            result.insert(0, 'I');
        }
    } else {
        result = "Unknown".to_string();
    }

    // 处理Rust关键字冲突
    if matches!(
        result.as_str(),
        "Self" | "self" | "super" | "crate" | "mod" | "type" | "fn" | "const" | "static"
    ) {
        result.push_str("Icon");
    }

    result
}

/// 校验图标尺寸一致性
fn validate_icon_sizes(icons: &[ProcessedIconInfo], category_name: &str) -> Result<()> {
    if icons.is_empty() {
        return Ok(());
    }

    let expected_size = icons[0].size;

    for icon in icons.iter().skip(1) {
        if icon.size != expected_size {
            bail!(
                "{}尺寸不一致: {}为{}x{}, 期望{}x{}",
                category_name,
                match &icon.source_type {
                    IconSourceType::Local { filename, .. } => filename,
                    IconSourceType::Weather { icon_code } => icon_code,
                },
                icon.size.width,
                icon.size.height,
                expected_size.width,
                expected_size.height
            );
        }
    }

    Ok(())
}

/// 合并和校验所有图标
fn merge_and_validate_icons(
    local_icons: &[ProcessedIconInfo],
    weather_icons: &[ProcessedIconInfo],
) -> Result<Vec<ProcessedIconInfo>> {
    let mut all_icons = Vec::new();
    all_icons.extend_from_slice(local_icons);
    all_icons.extend_from_slice(weather_icons);

    // 检查变体名唯一性
    let mut variant_names = HashSet::new();
    let mut duplicate_variants = Vec::new();

    for icon in &all_icons {
        if !variant_names.insert(&icon.variant_name) {
            duplicate_variants.push(icon.variant_name.clone());
        }
    }

    if !duplicate_variants.is_empty() {
        println!(
            "cargo:warning=Warning: 发现重复的变体名: {:?}",
            duplicate_variants,
        );

        // 为重复的变体名添加后缀
        let mut name_count = HashMap::new();
        for icon in &mut all_icons {
            let count = name_count.entry(icon.variant_name.clone()).or_insert(0);
            if *count > 0 {
                icon.variant_name = format!("{}{}", icon.variant_name, *count + 1);
            }
            *count += 1;
        }
    }

    Ok(all_icons)
}

/// 生成统一的图标Rust文件
fn generate_unified_icon_file(config: &BuildConfig, all_icons: &[ProcessedIconInfo]) -> Result<()> {
    let output_path = config.output_dir.join("generated_icons.rs");

    // 按来源类型分组图标
    let mut local_icons_by_category: HashMap<String, Vec<&ProcessedIconInfo>> = HashMap::new();
    let mut weather_icons = Vec::new();

    for icon in all_icons {
        match &icon.source_type {
            IconSourceType::Local { category, .. } => {
                local_icons_by_category
                    .entry(category.clone())
                    .or_default()
                    .push(icon);
            }
            IconSourceType::Weather { .. } => {
                weather_icons.push(icon);
            }
        }
    }

    // 首先，将每个分类的图标数据写入单独的bin文件
    for (category, icons) in &local_icons_by_category {
        if let Some(_category_config) = config
            .local_icon_categories
            .iter()
            .find(|c| c.category == *category)
        {
            let bin_path = config
                .output_dir
                .join(format!("generated_{}_icons.bin", category));

            // 将所有图标的位图数据连接起来
            let all_data: Vec<u8> = icons
                .iter()
                .flat_map(|icon| icon.bitmap_data.iter().copied())
                .collect();

            // 写入bin文件
            fs::write(&bin_path, &all_data)
                .with_context(|| format!("写入图标bin文件失败: {:?}", bin_path))?;
        }
    }

    // 如果存在天气图标，也将它们写入单独的bin文件
    if !weather_icons.is_empty() {
        let weather_bin_path = config.output_dir.join("generated_weather_icons.bin");

        let all_weather_data: Vec<u8> = weather_icons
            .iter()
            .flat_map(|icon| icon.bitmap_data.iter().copied())
            .collect();

        fs::write(&weather_bin_path, &all_weather_data)
            .with_context(|| format!("写入天气图标bin文件失败: {:?}", weather_bin_path))?;
    }

    let mut content = String::new();

    // 文件头部
    generate_file_header(&mut content, all_icons);

    // 生成枚举定义
    generate_enums(
        &mut content,
        config,
        &local_icons_by_category,
        &weather_icons,
    );

    // 生成图标数据和辅助方法
    generate_icon_data_and_methods(
        &mut content,
        config,
        &local_icons_by_category,
        &weather_icons,
    );

    // 写入文件
    file_utils::write_string_file(&output_path, &content)
        .with_context(|| format!("写入生成的图标文件失败: {:?}", output_path))?;

    println!(
        "cargo:warning=  生成统一图标文件成功: {} ({}个图标)",
        output_path.display(),
        all_icons.len()
    );

    Ok(())
}

/// 生成文件头部
fn generate_file_header(content: &mut String, _all_icons: &[ProcessedIconInfo]) {
    content.push_str("//! 生成的图标资源模块\n");
    content.push_str("//! 包含所有本地静态图标和天气图标定义\n");
    content.push_str("//! 不要手动修改此文件，由构建脚本自动生成\n\n");

    // 导入必要的依赖
    content.push_str("#![allow(dead_code, non_camel_case_types, non_upper_case_globals)]\n\n");
    content.push_str("use embedded_graphics::geometry::Size;\n\n");
    content.push_str("use crate::common::error::{AppError, Result};\n\n");
}

/// 生成枚举定义
fn generate_enums(
    content: &mut String,
    config: &BuildConfig,
    local_icons_by_category: &HashMap<String, Vec<&ProcessedIconInfo>>,
    weather_icons: &[&ProcessedIconInfo],
) {
    // 生成本地图标枚举
    for (category, icons) in local_icons_by_category {
        if let Some(category_config) = config
            .local_icon_categories
            .iter()
            .find(|c| c.category == *category)
        {
            content.push_str(&format!("/// {}图标枚举\n", category));
            content.push_str("#[derive(Copy, Clone, Debug, PartialEq, Eq)]\n");
            content.push_str(&format!("pub enum {} {{ \n", category_config.enum_name));

            for icon in icons {
                content.push_str(&format!(
                    "    {}, // {}\n",
                    icon.variant_name,
                    match &icon.source_type {
                        IconSourceType::Local { filename, .. } => filename,
                        _ => &icon.id,
                    }
                ));
            }

            content.push_str("}\n\n");
        }
    }

    // 生成天气图标枚举
    if !weather_icons.is_empty() {
        content.push_str("/// 天气图标枚举\n");
        content.push_str("#[derive(Copy, Clone, Debug, PartialEq, Eq)]\n");
        content.push_str(&format!(
            "pub enum {} {{\n",
            config.weather_icon_config.enum_name
        ));

        for icon in weather_icons {
            content.push_str(&format!("    {}, // {}\n", icon.variant_name, icon.id));
        }

        content.push_str("}\n\n");

        // 生成天气图标转换方法（删除了as_api_str方法）
        content.push_str(&format!(
            "impl {} {{\n",
            config.weather_icon_config.enum_name
        ));
        content.push_str("    /// 从API字符串获取天气图标\n");
        content.push_str("    pub fn from_api_str(s: &str) -> Result<Self> {\n");
        content.push_str("        match s {\n");

        for icon in weather_icons {
            if let IconSourceType::Weather { icon_code } = &icon.source_type {
                content.push_str(&format!(
                    "            \"{}\" => Ok(WeatherIcon::{}),\n",
                    icon_code, icon.variant_name
                ));
            }
        }

        content.push_str("            _ => Err(AppError::InvalidWeatherIconCode),\n");
        content.push_str("        }\n");
        content.push_str("    }\n");
        content.push_str("}\n\n");
    }

    // 生成统一的IconId枚举
    generate_icon_id_enum(
        content,
        config,
        local_icons_by_category,
        !weather_icons.is_empty(),
    );
}

/// 生成统一的IconId枚举
fn generate_icon_id_enum(
    content: &mut String,
    config: &BuildConfig,
    local_icons_by_category: &HashMap<String, Vec<&ProcessedIconInfo>>,
    has_weather_icons: bool,
) {
    content.push_str("/// 统一的图标ID枚举\n");
    content.push_str("/// 包含所有类型的图标\n");
    content.push_str("#[derive(Copy, Clone, Debug, PartialEq, Eq)]\n");
    content.push_str("pub enum IconId {\n");

    // 添加本地静态图标变体
    for (category, _) in local_icons_by_category {
        if let Some(category_config) = config
            .local_icon_categories
            .iter()
            .find(|c| c.category == *category)
        {
            content.push_str(&format!(
                "    {}({}),\n",
                category.to_ascii_uppercase(),
                category_config.enum_name
            ));
        }
    }

    // 添加天气图标变体
    if has_weather_icons {
        content.push_str("    Weather(WeatherIcon),\n");
    }

    content.push_str("}\n\n");
}

/// 生成图标数据和辅助方法
fn generate_icon_data_and_methods(
    content: &mut String,
    config: &BuildConfig,
    local_icons_by_category: &HashMap<String, Vec<&ProcessedIconInfo>>,
    weather_icons: &[&ProcessedIconInfo],
) {
    // 生成图标数据常量（使用include_bytes宏）
    for (category, icons) in local_icons_by_category {
        if let Some(_category_config) = config
            .local_icon_categories
            .iter()
            .find(|c| c.category == *category)
        {
            let category_upper = category.to_ascii_uppercase();

            // 生成位图数据引用（使用include_bytes）
            content.push_str(&format!("/// {}图标位图数据\n", category));
            content.push_str(&format!(
                "pub const {}_ICON_DATA: &[u8] = include_bytes!(\"generated_{}_icons.bin\");\n\n",
                category_upper, category
            ));

            // 生成图标尺寸
            if let Some(first_icon) = icons.first() {
                content.push_str(&format!("/// {}图标尺寸\n", category));
                content.push_str(&format!(
                    "pub const {}_ICON_SIZE: Size = Size {{\n",
                    category_upper
                ));
                content.push_str(&format!("    width: {},\n", first_icon.size.width));
                content.push_str(&format!("    height: {},\n", first_icon.size.height));
                content.push_str(&format!("}};\n\n"));
            }

            // 生成图标索引数量（用于后续计算）
            content.push_str(&format!("/// {}图标数量\n", category));
            content.push_str(&format!(
                "pub const {}_ICON_COUNT: usize = {};\n\n",
                category_upper,
                icons.len()
            ));

            // 生成每个图标的尺寸
            if let Some(first_icon) = icons.first() {
                content.push_str(&format!("/// {}每个图标的位图数据长度\n", category));
                content.push_str(&format!(
                    "pub const {}_ICON_BITMAP_LEN: usize = {};\n\n",
                    category_upper,
                    first_icon.size.bitmap_len()
                ));
            }
        }
    }

    // 生成天气图标数据
    if !weather_icons.is_empty() {
        content.push_str("/// 天气图标位图数据\n");
        content.push_str("pub const WEATHER_ICON_DATA: &[u8] = include_bytes!(\"generated_weather_icons.bin\");\n\n");

        // 生成天气图标尺寸
        if let Some(first_icon) = weather_icons.first() {
            content.push_str("/// 天气图标尺寸\n");
            content.push_str("pub const WEATHER_ICON_SIZE: Size = Size {\n");
            content.push_str(&format!("    width: {},\n", first_icon.size.width));
            content.push_str(&format!("    height: {},\n", first_icon.size.height));
            content.push_str(&format!("}};\n\n"));
        }

        // 生成天气图标数量
        content.push_str("/// 天气图标数量\n");
        content.push_str(&format!(
            "pub const WEATHER_ICON_COUNT: usize = {};\n\n",
            weather_icons.len()
        ));

        // 生成天气图标每个图标的位图数据长度
        if let Some(first_icon) = weather_icons.first() {
            content.push_str("/// 天气图标每个图标的位图数据长度\n");
            content.push_str(&format!(
                "pub const WEATHER_ICON_BITMAP_LEN: usize = {};\n\n",
                first_icon.size.bitmap_len()
            ));
        }
    }

    // 生成IconId的辅助方法
    generate_icon_id_methods(content, config, local_icons_by_category, weather_icons);
}

/// 生成IconId的方法实现
fn generate_icon_id_methods(
    content: &mut String,
    config: &BuildConfig,
    local_icons_by_category: &HashMap<String, Vec<&ProcessedIconInfo>>,
    weather_icons: &[&ProcessedIconInfo],
) {
    content.push_str("impl IconId {\n");

    // data方法（使用索引计算而不是常量数组）
    content.push_str("    /// 获取图标位图数据\n");
    content.push_str("    pub fn data(&self) -> &'static [u8] {\n");
    content.push_str("        match self {\n");

    for (category, icons) in local_icons_by_category {
        if let Some(category_config) = config
            .local_icon_categories
            .iter()
            .find(|c| c.category == *category)
        {
            let category_upper = category.to_ascii_uppercase();
            let enum_name = &category_config.enum_name;

            content.push_str(&format!(
                "            IconId::{}(icon) => {{\n",
                category_upper
            ));
            content.push_str(&format!("                let idx = match icon {{\n"));

            for (i, icon) in icons.iter().enumerate() {
                content.push_str(&format!(
                    "                    {}::{} => {},\n",
                    enum_name, icon.variant_name, i
                ));
            }

            content.push_str(&format!("                }};\n"));
            content.push_str(&format!(
                "                let start = idx * {}_ICON_BITMAP_LEN;\n",
                category_upper
            ));
            content.push_str(&format!(
                "                let end = start + {}_ICON_BITMAP_LEN;\n",
                category_upper
            ));
            content.push_str(&format!(
                "                &{}_ICON_DATA[start..end]\n",
                category_upper
            ));
            content.push_str("            }\n");
        }
    }

    if !weather_icons.is_empty() {
        content.push_str("            IconId::Weather(icon) => {\n");
        content.push_str("                let idx = match icon {\n");

        for (i, icon) in weather_icons.iter().enumerate() {
            content.push_str(&format!(
                "                    WeatherIcon::{} => {},\n",
                icon.variant_name, i
            ));
        }

        content.push_str("                };\n");
        content.push_str("                let start = idx * WEATHER_ICON_BITMAP_LEN;\n");
        content.push_str("                let end = start + WEATHER_ICON_BITMAP_LEN;\n");
        content.push_str("                &WEATHER_ICON_DATA[start..end]\n");
        content.push_str("            }\n");
    }

    content.push_str("        }\n");
    content.push_str("    }\n\n");

    // size方法
    content.push_str("    /// 获取图标尺寸\n");
    content.push_str("    pub fn size(&self) -> Size {\n");
    content.push_str("        match self {\n");

    for (category, icons) in local_icons_by_category {
        let category_upper = category.to_ascii_uppercase();

        if let Some(_) = icons.first() {
            content.push_str(&format!(
                "            IconId::{}(_) => {}_ICON_SIZE,\n",
                category_upper, category_upper
            ));
        }
    }

    if !weather_icons.is_empty() {
        content.push_str("            IconId::Weather(_) => WEATHER_ICON_SIZE,\n");
    }

    content.push_str("        }\n");
    content.push_str("    }\n");

    content.push_str("}\n");
}
