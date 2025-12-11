//! 字体生成模块 - 支持可配置的字体尺寸
//! 使用字体渲染器的返回结果，确保正确的字符映射

#![allow(unused)]

use crate::builder::config::BuildConfig;
use crate::builder::utils::font_renderer::{FontConfig, FontRenderer, GlyphMetrics};
use crate::builder::utils::{self, progress::ProgressTracker};
use anyhow::{Context, Result, anyhow};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

/// 字体尺寸配置
#[derive(Debug, Clone)]
pub struct FontSizeConfig {
    pub name: String, // 字体名称，如 "Small", "Medium", "Large"
    pub size: u32,    // 字体高度（像素）
}

impl FontSizeConfig {
    /// 创建新的字体尺寸配置
    pub fn new(name: &str, size: u32) -> Self {
        Self {
            name: name.to_string(),
            size,
        }
    }
}

/// 共享字符表 - 使用实际渲染成功的字符
#[derive(Debug)]
pub struct SharedCharset {
    pub chars: Vec<char>,   // 成功渲染的字符（已排序）
    pub missing: Vec<char>, // 缺失的字符
}

/// 字体位图数据
#[derive(Debug)]
pub struct FontBitmap {
    pub glyph_data: Vec<u8>,
    pub char_count: usize,                         // 成功渲染的字符数量
    pub missing_chars: Vec<char>,                  // 缺失的字符列表
    pub metrics_map: BTreeMap<char, GlyphMetrics>, // 字符到度量参数的映射
}

/// 构建字体数据（共享字符表）
pub fn build(config: &BuildConfig, progress: &ProgressTracker) -> Result<()> {
    progress.update_progress(0, 5, "读取字符集&去重排序");

    // 1. 读取并处理原始字符集
    let raw_charset = read_raw_charset(config)?;
    // println!(
    //     "cargo:warning=  原始字符集统计 - 共 {} 个字符（去重后）",
    //     raw_charset.len()
    // );

    // 2. 从构建配置中获取需要定义的字体尺寸
    let font_size_configs = config.font_size_configs.clone();

    // 3. 渲染第一个字体，建立共享字符表（作为基准）
    progress.update_progress(1, font_size_configs.len() + 3, "渲染基准字体（Small）");
    let baseline_bitmap = render_font_bitmap(config, &raw_charset, font_size_configs[0].clone())?;

    let mut shared_charset = SharedCharset {
        chars: baseline_bitmap.metrics_map.keys().cloned().collect(),
        missing: baseline_bitmap.missing_chars.clone(),
    };
    shared_charset.chars.sort(); // 确保字符有序

    // println!(
    //     "cargo:warning=  共享字符表统计 - 成功: {} 个字符，缺失: {} 个字符",
    //     shared_charset.chars.len(),
    //     shared_charset.missing.len()
    // );

    // 4. 渲染其他字体尺寸（使用共享字符表）
    let mut font_bitmaps = vec![baseline_bitmap];
    for (i, font_config) in font_size_configs.iter().skip(1).enumerate() {
        progress.update_progress(
            i + 2,
            font_size_configs.len() + 3,
            &format!("渲染{}字体", font_config.name),
        );

        let bitmap = render_font_bitmap(config, &shared_charset.chars, font_config.clone())?;
        font_bitmaps.push(bitmap);
    }

    // 5. 验证所有字体的字符一致性
    for (i, bitmap) in font_bitmaps.iter().enumerate() {
        let font_name = &font_size_configs[i].name;
        let current_chars: BTreeSet<char> = bitmap.metrics_map.keys().cloned().collect();
        let shared_set: BTreeSet<char> = shared_charset.chars.iter().cloned().collect();

        if current_chars != shared_set {
            let diff: Vec<char> = shared_set.difference(&current_chars).cloned().collect();
            return Err(anyhow!("{}字体缺少字符: {:?}", font_name, diff));
        }
    }

    // 6. 生成字体文件
    progress.update_progress(
        font_size_configs.len() + 3,
        font_size_configs.len() + 3,
        "生成字体二进制和Rust源文件",
    );
    generate_shared_font_files(config, &shared_charset, &font_size_configs, &font_bitmaps)?;

    // println!("cargo:warning=  字体生成完成 ✅");
    // println!(
    //     "cargo:warning=  生成字体尺寸: {}",
    //     font_size_configs
    //         .iter()
    //         .map(|f| format!("{} ({}px)", f.name, f.size))
    //         .collect::<Vec<_>>()
    //         .join(", ")
    // );

    Ok(())
}

/// 读取原始字符文件并去重排序
fn read_raw_charset(config: &BuildConfig) -> Result<Vec<char>> {
    let charset_path = config
        .font_path
        .parent()
        .ok_or_else(|| anyhow!("字体路径无父目录"))?
        .join("chars.txt");

    let content = fs::read_to_string(&charset_path)
        .with_context(|| format!("读取字符集文件失败: {}", charset_path.display()))?;

    // 使用 BTreeSet 自动去重和排序
    let mut char_set = BTreeSet::new();
    for c in content.chars() {
        // 过滤控制字符和空白字符
        if !c.is_control() && !c.is_whitespace() {
            char_set.insert(c);
        }
    }

    Ok(char_set.into_iter().collect())
}

/// 渲染字体位图数据（使用指定字符集）
fn render_font_bitmap(
    config: &BuildConfig,
    chars: &[char],
    font_config: FontSizeConfig,
) -> Result<FontBitmap> {
    let font_render_config = FontConfig {
        font_path: config.font_path.to_string_lossy().to_string(),
        font_size: font_config.size,
        chars: chars.to_vec(),
    };

    let render_result = FontRenderer::render_font(&font_render_config)
        .with_context(|| format!("渲染{}字体失败", font_config.name))?;

    // 验证渲染结果的一致性
    if render_result.char_mapping.len() != render_result.glyph_metrics_map.len() {
        return Err(anyhow!(
            "{}字体: 字符映射和度量参数数量不匹配",
            font_config.name
        ));
    }

    Ok(FontBitmap {
        glyph_data: render_result.glyph_data,
        char_count: render_result.rendered_chars,
        missing_chars: render_result.missing_chars,
        metrics_map: render_result.glyph_metrics_map,
    })
}

/// 生成共享字符表的字体文件
fn generate_shared_font_files(
    config: &BuildConfig,
    charset: &SharedCharset,
    font_configs: &[FontSizeConfig],
    font_bitmaps: &[FontBitmap],
) -> Result<()> {
    // 1. 生成二进制位图文件
    for (i, bitmap) in font_bitmaps.iter().enumerate() {
        let font_name = font_configs[i].name.to_lowercase();
        let bin_path = config
            .output_dir
            .join(format!("generated_{}_font.bin", font_name));

        write_font_bitmap(bitmap, &bin_path)?;
    }

    // 2. 生成Rust源文件
    generate_fonts_rs(config, charset, font_configs, font_bitmaps)?;

    Ok(())
}

/// 写入位图数据到文件并输出统计信息
fn write_font_bitmap(bitmap: &FontBitmap, path: &Path) -> Result<()> {
    fs::write(path, &bitmap.glyph_data)
        .with_context(|| format!("写入字体文件失败: {}", path.display()))?;

    // 计算统计信息
    // let total_size_kb = bitmap.glyph_data.len() as f64 / 1024.0;
    // let avg_char_size = if bitmap.char_count > 0 {
    //     bitmap.glyph_data.len() as f64 / bitmap.char_count as f64
    // } else {
    //     0.0
    // };

    // println!(
    //     "cargo:warning=  生成字体文件: {} | 大小: {:.2}KB | 字符数: {} | 平均字符大小: {:.2}字节/字符",
    //     path.display(),
    //     total_size_kb,
    //     bitmap.char_count,
    //     avg_char_size
    // );

    Ok(())
}

/// 生成字体数据Rust源文件
fn generate_fonts_rs(
    config: &BuildConfig,
    charset: &SharedCharset,
    font_configs: &[FontSizeConfig],
    font_bitmaps: &[FontBitmap],
) -> Result<()> {
    let output_path = config.output_dir.join("generated_fonts.rs");
    let mut content = String::new();

    // 头部注释
    content.push_str("//! 自动生成的字体数据文件（含字形度量参数）\n");
    content.push_str("//! 不要手动修改此文件\n");
    content.push_str("\n\n");
    content.push_str("#![allow(dead_code)]\n");
    content.push_str("#![allow(clippy::unreadable_literal)]\n\n");

    content.push_str("use serde::{Deserialize, Serialize};\n\n");

    // 定义GlyphMetrics结构体（与渲染器一致）
    content.push_str("// ==================== 核心结构体定义 ====================\n");
    content.push_str("/// 单个字符的字形度量参数\n");
    content.push_str("#[derive(Debug, Clone, Copy, PartialEq, Eq)]\n");
    content.push_str("pub struct GlyphMetrics {\n");
    content.push_str("    /// 字符在bin文件中的起始偏移（字节）\n");
    content.push_str("    pub offset: u32,\n");
    content.push_str("    /// 字符位图宽度（像素）\n");
    content.push_str("    pub width: u32,\n");
    content.push_str("    /// 字符位图高度（像素）\n");
    content.push_str("    pub height: u32,\n");
    content.push_str("    /// 水平偏移（BearingX）：字符位图相对基线的X偏移（像素）\n");
    content.push_str("    pub bearing_x: i32,\n");
    content.push_str("    /// 垂直偏移（BearingY）：字符位图相对基线的Y偏移（像素）\n");
    content.push_str("    pub bearing_y: i32,\n");
    content.push_str("    /// 水平Advance（AdvanceX）：字符渲染后的X轴移动距离（像素）\n");
    content.push_str("    pub advance_x: i32,\n");
    content.push_str("}\n\n");

    // 共享字符表
    content.push_str("// ==================== 共享字符表 ====================\n");
    content.push_str("/// 字符表（已排序，所有字体共享）\n");
    content.push_str("pub const CHARS: &[char] = &[\n");
    for (i, &c) in charset.chars.iter().enumerate() {
        // 每10个字符换行，提高可读性
        if i % 10 == 0 && i > 0 {
            content.push_str("\n");
        }
        // 转义特殊字符
        let c_escaped = match c {
            '\'' => "\\'".to_string(),
            '\\' => "\\\\".to_string(),
            _ => c.to_string(),
        };
        content.push_str(&format!("'{c_escaped}', "));
    }
    content.push_str("\n];\n\n");

    content.push_str(&format!(
        "/// 总字符数量\npub const CHAR_COUNT: usize = {};\n\n",
        charset.chars.len()
    ));

    content.push_str(&format!(
        "/// 缺失的字符列表\npub const MISSING_CHARS: &[char] = &[\n"
    ));
    for (i, &c) in charset.missing.iter().enumerate() {
        if i % 10 == 0 && i > 0 {
            content.push_str("\n");
        }
        let c_escaped = match c {
            '\'' => "\\'".to_string(),
            '\\' => "\\\\".to_string(),
            _ => c.to_string(),
        };
        content.push_str(&format!("'{c_escaped}', "));
    }
    content.push_str("\n];\n\n");

    // 生成各字体的度量参数数组
    content.push_str("// ==================== 字体度量参数 ====================\n");
    for (i, font_config) in font_configs.iter().enumerate() {
        let bitmap = &font_bitmaps[i];
        let name_upper = font_config.name.to_uppercase();
        let name_lower = font_config.name.to_lowercase();

        content.push_str(&format!(
            "// {}字体 ({}px) 度量参数\n",
            font_config.name, font_config.size
        ));
        content.push_str(&format!(
            "pub const FONT_{name_upper}_METRICS: &[GlyphMetrics] = &[\n"
        ));

        // 按共享字符表顺序生成度量参数
        for &c in &charset.chars {
            let metrics = bitmap
                .metrics_map
                .get(&c)
                .ok_or_else(|| anyhow!("{}字体缺少字符 '{}' 的度量参数", font_config.name, c))?;

            content.push_str(&format!("    // 字符 '{}' (U+{:04X})\n", c, c as u32));
            content.push_str(&format!(
                "    GlyphMetrics {{
        offset: {},
        width: {},
        height: {},
        bearing_x: {},
        bearing_y: {},
        advance_x: {},
    }},\n",
                metrics.offset,
                metrics.width,
                metrics.height,
                metrics.bearing_x,
                metrics.bearing_y,
                metrics.advance_x
            ));
        }
        content.push_str("];\n\n");

        // 嵌入二进制位图数据
        content.push_str(&format!("// {}字体位图数据\n", font_config.name));
        content.push_str(&format!(
            "pub const FONT_{name_upper}_BITMAP: &[u8] = include_bytes!(\"generated_{name_lower}_font.bin\");\n\n"
        ));
    }

    // 字体尺寸枚举
    content.push_str("// ==================== 字体尺寸枚举 ====================\n");
    content.push_str("/// 字体尺寸选项\n");
    content.push_str("#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]\n");
    content.push_str("pub enum FontSize {\n");
    for font_config in font_configs {
        content.push_str(&format!(
            "    /// {}字体 ({}px)\n",
            font_config.name, font_config.size
        ));
        content.push_str(&format!("    {},\n", font_config.name));
    }
    content.push_str("}\n\n");

    // 辅助函数
    content.push_str("// ==================== 辅助函数 ====================\n");
    // 二分查找字符索引
    content.push_str("/// 二分查找字符在共享字符表中的索引\n");
    content.push_str("#[inline(always)]\n");
    content.push_str("pub fn find_char_index(c: char) -> Option<usize> {\n");
    content.push_str("    CHARS.binary_search(&c).ok()\n");
    content.push_str("}\n\n");

    // FontSize方法实现
    content.push_str("impl FontSize {\n");
    // 获取字体像素尺寸
    content.push_str("    /// 获取字体的像素高度\n");
    content.push_str("    pub const fn pixel_size(self) -> u32 {\n");
    content.push_str("        match self {\n");
    for font_config in font_configs {
        content.push_str(&format!(
            "            Self::{} => {},\n",
            font_config.name, font_config.size
        ));
    }
    content.push_str("        }\n");
    content.push_str("    }\n\n");

    // 获取字符度量参数
    content.push_str("    /// 获取字符的字形度量参数\n");
    content.push_str("    pub fn get_glyph_metrics(self, c: char) -> Option<GlyphMetrics> {\n");
    content.push_str("        let idx = find_char_index(c)?;\n");
    content.push_str("        match self {\n");
    for font_config in font_configs {
        let name_upper = font_config.name.to_uppercase();
        content.push_str(&format!(
            "            Self::{} => FONT_{name_upper}_METRICS.get(idx).copied(),\n",
            font_config.name
        ));
    }
    content.push_str("        }\n");
    content.push_str("    }\n\n");

    // 获取字符位图数据
    content.push_str("    /// 获取字符的二值位图数据\n");
    content.push_str("    pub fn get_glyph_bitmap(self, c: char) -> Option<&'static [u8]> {\n");
    content.push_str("        let metrics = self.get_glyph_metrics(c)?;\n");
    content.push_str("        let bitmap_data = match self {\n");
    for font_config in font_configs {
        let name_upper = font_config.name.to_uppercase();
        content.push_str(&format!(
            "            Self::{} => FONT_{name_upper}_BITMAP,\n",
            font_config.name
        ));
    }
    content.push_str("        };\n");
    content.push_str("        \n");
    content.push_str("        // 计算字符数据长度（二值图像：(width + 7)/8 * height）\n");
    content.push_str("        let bytes_per_row = (metrics.width + 7) / 8;\n");
    content.push_str("        let data_len = (bytes_per_row * metrics.height) as usize;\n");
    content.push_str("        let start = metrics.offset as usize;\n");
    content.push_str("        let end = start + data_len;\n");
    content.push_str("        \n");
    content.push_str("        if end > bitmap_data.len() {\n");
    content.push_str("            None\n");
    content.push_str("        } else {\n");
    content.push_str("            Some(&bitmap_data[start..end])\n");
    content.push_str("        }\n");
    content.push_str("    }\n");
    content.push_str("}\n");

    // 写入文件
    utils::file_utils::write_string_file(&output_path, &content)
        .with_context(|| format!("写入Rust字体文件失败: {}", output_path.display()))?;

    let output_path = config.shared_output_dir.join("generated_font_size.rs");
    let mut content = String::new();

    content.push_str("use serde::{Deserialize, Serialize};\n\n");

    content.push_str("// ==================== 字体尺寸枚举 ====================\n");
    content.push_str("/// 字体尺寸选项\n");
    content.push_str("#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]\n");
    content.push_str("pub enum FontSize {\n");
    for font_config in font_configs {
        content.push_str(&format!(
            "    /// {}字体 ({}px)\n",
            font_config.name, font_config.size
        ));
        content.push_str(&format!("    {},\n", font_config.name));
    }
    content.push_str("}\n\n");

    content.push_str("impl TryFrom<&str> for FontSize {\n");
    content.push_str("    type Error = String;\n");
    content.push_str("    fn try_from(s: &str) -> Result<Self, Self::Error> {\n");
    content.push_str("        match s.to_lowercase().as_str() {\n");
    for font_config in font_configs {
        content.push_str(&format!(
            "            \"{}\" => Ok(Self::{}),\n",
            font_config.name, font_config.name
        ));
    }
    content.push_str("            _ => Err(format!(\"无效的字体尺寸: {}\", s)),\n");
    content.push_str("        }\n");
    content.push_str("    }\n");
    content.push_str("}\n\n");

    // 写入文件
    utils::file_utils::write_string_file(&output_path, &content)
        .with_context(|| format!("写入Rust字体文件失败: {}", output_path.display()))?;

    // println!(
    //     "cargo:warning=  生成Rust字体描述文件: {}",
    //     output_path.display()
    // );

    Ok(())
}
