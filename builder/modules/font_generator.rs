//! 字体生成模块 - 支持可配置的字体尺寸

use crate::builder::config::BuildConfig;
use crate::builder::utils::font_renderer::{FontConfig, FontRenderer};
use crate::builder::utils::{self, progress::ProgressTracker};
use anyhow::{Context, Result};
use std::collections::BTreeSet;
use std::fs;

/// 字体尺寸配置
#[derive(Debug, Clone)]
pub struct FontSizeConfig {
    pub name: String,           // 字体名称，如 "Small", "Medium", "Large"
    pub size: u32,              // 字体高度（像素）
    pub full_width_pixels: u32, // 全角字符宽度（像素）
    pub half_width_pixels: u32, // 半角字符宽度（像素）
}

impl FontSizeConfig {
    /// 创建新的字体尺寸配置
    pub fn new(name: &str, size: u32) -> Self {
        Self {
            name: name.to_string(),
            size,
            full_width_pixels: size,     // 全角字符为正方形
            half_width_pixels: size / 2, // 半角字符宽度为高度的一半
        }
    }

    /// 计算每行字节数
    pub fn bytes_per_row(&self, is_full_width: bool) -> u8 {
        let width = if is_full_width {
            self.full_width_pixels
        } else {
            self.half_width_pixels
        };
        ((width + 7) / 8) as u8
    }

    /// 计算每个字符的字节大小
    pub fn glyph_size(&self, is_full_width: bool) -> usize {
        let bytes_per_row = self.bytes_per_row(is_full_width) as usize;
        let height = self.size as usize;
        bytes_per_row * height
    }
}

/// 共享字符表
#[derive(Debug)]
pub struct SharedCharset {
    pub full_width: Vec<char>,
    pub half_width: Vec<char>,
}

/// 字体位图数据
#[derive(Debug)]
pub struct FontBitmap {
    pub glyph_data: Vec<u8>,
    pub config: FontSizeConfig,
    pub is_full_width: bool,
}

/// 构建字体数据（共享字符表）
pub fn build(config: &BuildConfig, progress: &ProgressTracker) -> Result<()> {
    progress.update_progress(0, 5, "读取字符集&分离和排序字符");
    let shared_charset = load_charset(config)?;

    println!(
        "cargo:warning=  字符表统计 - 全角: {}, 半角: {}",
        shared_charset.full_width.len(),
        shared_charset.half_width.len()
    );

    // 定义字体尺寸配置
    let font_configs = vec![
        FontSizeConfig::new("Small", 16),  // 小号字体 16px
        FontSizeConfig::new("Medium", 24), // 中号字体 24px
        FontSizeConfig::new("Large", 40),  // 大号字体 40px
    ];

    let mut font_bitmaps = Vec::new();

    // 为每个字体配置渲染全角和半角字符
    for (i, font_config) in font_configs.iter().enumerate() {
        progress.update_progress(
            i + 2,
            font_configs.len() + 2,
            &format!("渲染{}字体", font_config.name),
        );

        // 渲染全角字符
        let full_bitmap = render_font_bitmap(
            config,
            &shared_charset.full_width,
            font_config.clone(),
            false,
        )?;

        // 渲染半角字符
        let half_bitmap = render_font_bitmap(
            config,
            &shared_charset.half_width,
            font_config.clone(),
            true,
        )?;

        font_bitmaps.push((full_bitmap, half_bitmap));
    }

    progress.update_progress(
        font_configs.len() + 2,
        font_configs.len() + 2,
        "生成字体文件",
    );
    generate_shared_font_files(config, &shared_charset, &font_configs, &font_bitmaps)?;

    progress.update_progress(font_configs.len() + 3, font_configs.len() + 2, "完成");

    println!("cargo:warning=  字体生成完成");

    Ok(())
}

/// 加载字符集并创建共享字符表
pub fn load_charset(config: &BuildConfig) -> Result<SharedCharset> {
    let charset_path = config.font_path.parent().unwrap().join("chars.txt");

    let content = fs::read_to_string(&charset_path)
        .with_context(|| format!("读取字符集文件失败: {}", charset_path.display()))?;

    // 使用 BTreeSet 自动去重和排序
    let mut char_set = BTreeSet::new();
    for c in content.chars() {
        if !c.is_control() && !c.is_whitespace() {
            char_set.insert(c);
        }
    }

    // 分离全角和半角字符
    let mut full_width = Vec::new();
    let mut half_width = Vec::new();

    for &c in &char_set {
        if c.is_ascii() && c.is_ascii_graphic() {
            half_width.push(c);
        } else {
            full_width.push(c);
        }
    }

    // 确保半角字符包含完整的ASCII可打印字符集
    for code in 0x20..=0x7E {
        if let Some(c) = std::char::from_u32(code) {
            if c.is_ascii_graphic() || c == ' ' {
                if !half_width.contains(&c) {
                    half_width.push(c);
                }
            }
        }
    }

    // 排序
    full_width.sort();
    half_width.sort();

    // 验证排序
    for i in 1..full_width.len() {
        if full_width[i] <= full_width[i - 1] {
            println!(
                "cargo:warning=警告: 全角字符未正确排序: {:?} <= {:?}",
                full_width[i],
                full_width[i - 1]
            );
        }
    }

    Ok(SharedCharset {
        full_width,
        half_width,
    })
}

/// 渲染字体位图数据（返回纯位图数据，不包含字符表）
fn render_font_bitmap(
    config: &BuildConfig,
    chars: &[char],
    font_config: FontSizeConfig,
    is_half_width: bool,
) -> Result<FontBitmap> {
    let font_render_config = FontConfig {
        font_path: config.font_path.to_string_lossy().to_string(),
        font_size: font_config.size,
        is_half_width,
        chars: chars.to_vec(),
    };

    let render_result = FontRenderer::render_font(&font_render_config)?;

    Ok(FontBitmap {
        glyph_data: render_result.glyph_data,
        config: font_config,
        is_full_width: !is_half_width,
    })
}

/// 生成共享字符表的字体文件
fn generate_shared_font_files(
    config: &BuildConfig,
    charset: &SharedCharset,
    font_configs: &[FontSizeConfig],
    font_bitmaps: &[(FontBitmap, FontBitmap)],
) -> Result<()> {
    // 1. 生成二进制文件
    for (i, (full_bitmap, half_bitmap)) in font_bitmaps.iter().enumerate() {
        let font_name = &font_configs[i].name.to_lowercase();

        write_font_bitmap(
            full_bitmap,
            &config
                .output_dir
                .join(format!("generated_{}_full_width_font.bin", font_name)),
        )?;

        write_font_bitmap(
            half_bitmap,
            &config
                .output_dir
                .join(format!("generated_{}_half_width_font.bin", font_name)),
        )?;
    }

    // 2. 生成Rust源文件
    generate_fonts_rs(config, charset, font_configs)?;

    Ok(())
}

/// 写入位图数据到文件
fn write_font_bitmap(bitmap: &FontBitmap, path: &std::path::Path) -> Result<()> {
    std::fs::write(path, &bitmap.glyph_data)?;

    let glyph_size = bitmap.config.glyph_size(bitmap.is_full_width);
    let width = if bitmap.is_full_width {
        bitmap.config.full_width_pixels
    } else {
        bitmap.config.half_width_pixels
    };

    println!(
        "cargo:warning=  生成字体文件: {}, 大小: {}KB (每个字符{}字节, {}x{}像素)",
        path.display(),
        bitmap.glyph_data.len() / 1024,
        glyph_size,
        width,
        bitmap.config.size
    );

    Ok(())
}

/// 生成字体数据Rust源文件
fn generate_fonts_rs(
    config: &BuildConfig,
    charset: &SharedCharset,
    font_configs: &[FontSizeConfig],
) -> Result<()> {
    let output_path = config.output_dir.join("generated_fonts.rs");

    let mut content = String::new();

    // 头部注释
    content.push_str("//! 自动生成的字体数据文件（共享字符表）\n");
    content.push_str("//! 不要手动修改此文件\n\n");
    content.push_str("#![allow(dead_code, non_upper_case_globals)]\n\n");

    // ========== 1. 共享字符表 ==========
    content.push_str("// ==================== 共享字符表 ====================\n\n");

    // 全角字符表
    content.push_str("/// 全角字符表（已排序）\n");
    content.push_str("pub const FULL_WIDTH_CHARS: &[u16] = &[\n");
    for (i, &c) in charset.full_width.iter().enumerate() {
        if i % 10 == 0 && i > 0 {
            content.push_str("\n");
        }
        content.push_str(&format!("0x{:04X}, ", c as u16));
    }
    content.push_str("\n];\n\n");

    content.push_str(&format!(
        "/// 全角字符数量\npub const FULL_WIDTH_CHAR_COUNT: usize = {};\n\n",
        charset.full_width.len()
    ));

    // 半角字符表
    content.push_str("/// 半角字符表（已排序）\n");
    content.push_str("pub const HALF_WIDTH_CHARS: &[u16] = &[\n");
    for (i, &c) in charset.half_width.iter().enumerate() {
        if i % 10 == 0 && i > 0 {
            content.push_str("\n");
        }
        content.push_str(&format!("0x{:04X}, ", c as u16));
    }
    content.push_str("\n];\n\n");

    content.push_str(&format!(
        "/// 半角字符数量\npub const HALF_WIDTH_CHAR_COUNT: usize = {};\n\n",
        charset.half_width.len()
    ));

    // ========== 2. 字体常量 ==========
    content.push_str("// ==================== 字体常量 ====================\n\n");

    // 为每个字体配置生成常量
    for font_config in font_configs {
        let name_upper = font_config.name.to_uppercase();

        // 全角字体常量
        content.push_str(&format!(
            "// {}字体 ({}px)\n",
            font_config.name, font_config.size
        ));
        content.push_str(&format!(
            "pub const FONT_{}_FULL_WIDTH: u8 = {};\n",
            name_upper, font_config.full_width_pixels
        ));
        content.push_str(&format!(
            "pub const FONT_{}_FULL_HEIGHT: u8 = {};\n",
            name_upper, font_config.size
        ));
        content.push_str(&format!(
            "pub const FONT_{}_FULL_BYTES_PER_ROW: u8 = {}; // ({} + 7) / 8\n",
            name_upper,
            font_config.bytes_per_row(true),
            font_config.full_width_pixels
        ));
        content.push_str(&format!(
            "pub const FONT_{}_FULL_GLYPH_SIZE: usize = {}; // {} * {}\n\n",
            name_upper,
            font_config.glyph_size(true),
            font_config.bytes_per_row(true),
            font_config.size
        ));

        // 半角字体常量
        content.push_str(&format!(
            "pub const FONT_{}_HALF_WIDTH: u8 = {};\n",
            name_upper, font_config.half_width_pixels
        ));
        content.push_str(&format!(
            "pub const FONT_{}_HALF_HEIGHT: u8 = {};\n",
            name_upper, font_config.size
        ));
        content.push_str(&format!(
            "pub const FONT_{}_HALF_BYTES_PER_ROW: u8 = {}; // ({} + 7) / 8\n",
            name_upper,
            font_config.bytes_per_row(false),
            font_config.half_width_pixels
        ));
        content.push_str(&format!(
            "pub const FONT_{}_HALF_GLYPH_SIZE: usize = {}; // {} * {}\n\n",
            name_upper,
            font_config.glyph_size(false),
            font_config.bytes_per_row(false),
            font_config.size
        ));
    }

    // ========== 3. 位图数据 ==========
    content.push_str("// ==================== 位图数据 ====================\n\n");

    // 使用 include_bytes! 嵌入二进制数据
    for font_config in font_configs {
        let name_lower = font_config.name.to_lowercase();

        content.push_str(&format!("// {}字体位图\n", font_config.name));
        content.push_str(&format!("pub const {}_FULL_WIDTH_BITMAP: &[u8] = include_bytes!(\"generated_{}_full_width_font.bin\");\n", 
            name_lower.to_uppercase(), name_lower));
        content.push_str(&format!("pub const {}_HALF_WIDTH_BITMAP: &[u8] = include_bytes!(\"generated_{}_half_width_font.bin\");\n\n", 
            name_lower.to_uppercase(), name_lower));
    }

    // ========== 4. 辅助函数 ==========
    content.push_str("// ==================== 辅助函数 ====================\n\n");

    // 二分查找函数
    content.push_str("/// 二分查找字符索引\n");
    content.push_str("#[inline(always)]\n");
    content.push_str("fn find_char_index(chars: &[u16], target: u16) -> Option<usize> {\n");
    content.push_str("    let mut left = 0;\n");
    content.push_str("    let mut right = chars.len();\n");
    content.push_str("    \n");
    content.push_str("    while left < right {\n");
    content.push_str("        let mid = left + (right - left) / 2;\n");
    content.push_str("        if chars[mid] < target {\n");
    content.push_str("            left = mid + 1;\n");
    content.push_str("        } else if chars[mid] > target {\n");
    content.push_str("            right = mid;\n");
    content.push_str("        } else {\n");
    content.push_str("            return Some(mid);\n");
    content.push_str("        }\n");
    content.push_str("    }\n");
    content.push_str("    None\n");
    content.push_str("}\n\n");

    // 字体尺寸枚举
    content.push_str("/// 字体尺寸\n");
    content.push_str("#[derive(Copy, Clone, Debug, PartialEq, Eq)]\n");
    content.push_str("pub enum FontSize {\n");
    for font_config in font_configs {
        content.push_str(&format!("    {},\n", font_config.name));
    }
    content.push_str("}\n\n");

    // 字符宽度类型枚举
    content.push_str("/// 字符宽度类型\n");
    content.push_str("#[derive(Copy, Clone, Debug, PartialEq, Eq)]\n");
    content.push_str("pub enum CharWidth {\n");
    content.push_str("    Full,\n");
    content.push_str("    Half,\n");
    content.push_str("}\n\n");

    // 主要字形获取函数
    content.push_str("/// 获取字形数据\n");
    content.push_str("/// \n");
    content.push_str("/// # 参数\n");
    content.push_str("/// - `c`: 要查找的字符\n");
    content.push_str("/// - `size`: 字体尺寸\n");
    content.push_str("/// - `width_type`: 字符宽度类型\n");
    content.push_str("/// \n");
    content.push_str("/// # 返回值\n");
    content.push_str("/// 如果找到字符，返回字形数据的切片；否则返回None\n");
    content.push_str("pub fn get_glyph(c: char, size: FontSize, width_type: CharWidth) -> Option<&'static [u8]> {\n");
    content.push_str("    let target = c as u16;\n");
    content.push_str("    \n");
    content.push_str("    match width_type {\n");
    content.push_str("        CharWidth::Full => {\n");
    content.push_str("            let idx = find_char_index(FULL_WIDTH_CHARS, target)?;\n");
    content.push_str("            match size {\n");

    for (_, font_config) in font_configs.iter().enumerate() {
        let name_upper = font_config.name.to_uppercase();

        content.push_str(&format!(
            "                FontSize::{} => {{\n",
            font_config.name
        ));
        content.push_str(&format!(
            "                    let start = idx * FONT_{}_FULL_GLYPH_SIZE;\n",
            name_upper
        ));
        content.push_str(&format!("                    Some(&{}_FULL_WIDTH_BITMAP[start..start + FONT_{}_FULL_GLYPH_SIZE])\n", 
            name_upper, name_upper));
        content.push_str("                }\n");
    }

    content.push_str("            }\n");
    content.push_str("        }\n");
    content.push_str("        CharWidth::Half => {\n");
    content.push_str("            let idx = find_char_index(HALF_WIDTH_CHARS, target)?;\n");
    content.push_str("            match size {\n");

    for (_, font_config) in font_configs.iter().enumerate() {
        let name_upper = font_config.name.to_uppercase();

        content.push_str(&format!(
            "                FontSize::{} => {{\n",
            font_config.name
        ));
        content.push_str(&format!(
            "                    let start = idx * FONT_{}_HALF_GLYPH_SIZE;\n",
            name_upper
        ));
        content.push_str(&format!("                    Some(&{}_HALF_WIDTH_BITMAP[start..start + FONT_{}_HALF_GLYPH_SIZE])\n", 
            name_upper, name_upper));
        content.push_str("                }\n");
    }

    content.push_str("            }\n");
    content.push_str("        }\n");
    content.push_str("    }\n");
    content.push_str("}\n\n");

    // 自动判断字符宽度的函数
    content.push_str("/// 自动判断字符宽度并获取字形\n");
    content.push_str("/// \n");
    content.push_str("/// ASCII字符使用半角宽度，其他字符使用全角宽度\n");
    content.push_str("pub fn get_glyph_auto(c: char, size: FontSize) -> Option<&'static [u8]> {\n");
    content.push_str("    let width_type = if c.is_ascii() && c.is_ascii_graphic() {\n");
    content.push_str("        CharWidth::Half\n");
    content.push_str("    } else {\n");
    content.push_str("        CharWidth::Full\n");
    content.push_str("    };\n");
    content.push_str("    get_glyph(c, size, width_type)\n");
    content.push_str("}\n\n");

    // 字体信息查询函数
    content.push_str("/// 获取字体尺寸信息\n");
    content.push_str(
        "pub fn get_font_metrics(size: FontSize, is_full_width: bool) -> (u8, u8, u8) {\n",
    );
    content.push_str("    match (size, is_full_width) {\n");

    for (_, font_config) in font_configs.iter().enumerate() {
        let name_upper = font_config.name.to_uppercase();

        // 全角
        content.push_str(&format!("        (FontSize::{}, true) => (FONT_{}_FULL_WIDTH, FONT_{}_FULL_HEIGHT, FONT_{}_FULL_BYTES_PER_ROW),\n", 
            font_config.name, name_upper, name_upper, name_upper));

        // 半角
        content.push_str(&format!("        (FontSize::{}, false) => (FONT_{}_HALF_WIDTH, FONT_{}_HALF_HEIGHT, FONT_{}_HALF_BYTES_PER_ROW),\n", 
            font_config.name, name_upper, name_upper, name_upper));
    }

    content.push_str("    }\n");
    content.push_str("}\n\n");

    // 写入文件
    utils::file_utils::write_string_file(&output_path, &content)?;

    println!(
        "cargo:warning=  生成字体描述文件: {}",
        output_path.display()
    );

    Ok(())
}
