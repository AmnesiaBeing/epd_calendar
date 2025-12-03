//! 字体生成模块 - 支持3种不同大小的字符集

use crate::builder::config::BuildConfig;
use crate::builder::utils::font_renderer::{FontConfig, FontRenderer};
use crate::builder::utils::{self, progress::ProgressTracker};
use anyhow::{Context, Result};
use std::collections::BTreeSet;
use std::fs;

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
    pub size: u32,
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

    progress.update_progress(2, 5, "渲染小号字体");
    let small_full = render_font_bitmap(config, &shared_charset.full_width, 12, false)?;
    let small_half = render_font_bitmap(config, &shared_charset.half_width, 12, true)?;

    progress.update_progress(3, 5, "渲染中号字体");
    let medium_full = render_font_bitmap(config, &shared_charset.full_width, 16, false)?;
    let medium_half = render_font_bitmap(config, &shared_charset.half_width, 16, true)?;

    progress.update_progress(4, 5, "渲染大号字体");
    let large_full = render_font_bitmap(config, &shared_charset.full_width, 24, false)?;
    let large_half = render_font_bitmap(config, &shared_charset.half_width, 24, true)?;

    progress.update_progress(5, 5, "生成字体文件");
    generate_shared_font_files(
        config,
        &shared_charset,
        &small_full,
        &small_half,
        &medium_full,
        &medium_half,
        &large_full,
        &large_half,
    )?;

    progress.update_progress(6, 5, "完成");

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
    size: u32,
    is_half_width: bool,
) -> Result<FontBitmap> {
    let font_config = FontConfig {
        font_path: config.font_path.to_string_lossy().to_string(),
        font_size: size,
        is_half_width,
        chars: chars.to_vec(),
    };

    let render_result = FontRenderer::render_font(&font_config)?;

    Ok(FontBitmap {
        glyph_data: render_result.glyph_data,
        size,
        is_full_width: !is_half_width,
    })
}

/// 生成共享字符表的字体文件
fn generate_shared_font_files(
    config: &BuildConfig,
    charset: &SharedCharset,
    small_full: &FontBitmap,
    small_half: &FontBitmap,
    medium_full: &FontBitmap,
    medium_half: &FontBitmap,
    large_full: &FontBitmap,
    large_half: &FontBitmap,
) -> Result<()> {
    // 1. 生成二进制文件
    write_font_bitmap(
        small_full,
        &config
            .output_dir
            .join("generated_small_full_width_font.bin"),
    )?;
    write_font_bitmap(
        small_half,
        &config
            .output_dir
            .join("generated_small_half_width_font.bin"),
    )?;
    write_font_bitmap(
        medium_full,
        &config
            .output_dir
            .join("generated_medium_full_width_font.bin"),
    )?;
    write_font_bitmap(
        medium_half,
        &config
            .output_dir
            .join("generated_medium_half_width_font.bin"),
    )?;
    write_font_bitmap(
        large_full,
        &config
            .output_dir
            .join("generated_large_full_width_font.bin"),
    )?;
    write_font_bitmap(
        large_half,
        &config
            .output_dir
            .join("generated_large_half_width_font.bin"),
    )?;

    // 2. 生成Rust源文件
    generate_fonts_rs(config, charset)?;

    Ok(())
}

/// 写入位图数据到文件
fn write_font_bitmap(bitmap: &FontBitmap, path: &std::path::Path) -> Result<()> {
    std::fs::write(path, &bitmap.glyph_data)?;

    let glyph_size = if bitmap.is_full_width {
        match bitmap.size {
            12 => 24, // 2 * 12
            16 => 32, // 2 * 16
            24 => 72, // 3 * 24
            _ => 32,
        }
    } else {
        match bitmap.size {
            12 => 12, // 1 * 12
            16 => 16, // 1 * 16
            24 => 48, // 2 * 24
            _ => 16,
        }
    };

    println!(
        "cargo:warning=  生成字体文件: {}, 大小: {}KB (每个字符{}字节)",
        path.display(),
        bitmap.glyph_data.len() / 1024,
        glyph_size
    );

    Ok(())
}

/// 生成字体数据Rust源文件
fn generate_fonts_rs(config: &BuildConfig, charset: &SharedCharset) -> Result<()> {
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

    // 小号字体常量
    content.push_str("// 小号字体 (12px)\n");
    content.push_str("pub const FONT_SMALL_FULL_WIDTH: u8 = 12;\n");
    content.push_str("pub const FONT_SMALL_FULL_HEIGHT: u8 = 12;\n");
    content.push_str("pub const FONT_SMALL_FULL_BYTES_PER_ROW: u8 = 2; // (12 + 7) / 8\n");
    content.push_str("pub const FONT_SMALL_FULL_GLYPH_SIZE: usize = 24; // 2 * 12\n\n");

    content.push_str("pub const FONT_SMALL_HALF_WIDTH: u8 = 6;\n");
    content.push_str("pub const FONT_SMALL_HALF_HEIGHT: u8 = 12;\n");
    content.push_str("pub const FONT_SMALL_HALF_BYTES_PER_ROW: u8 = 1; // (6 + 7) / 8\n");
    content.push_str("pub const FONT_SMALL_HALF_GLYPH_SIZE: usize = 12; // 1 * 12\n\n");

    // 中号字体常量
    content.push_str("// 中号字体 (16px)\n");
    content.push_str("pub const FONT_MEDIUM_FULL_WIDTH: u8 = 16;\n");
    content.push_str("pub const FONT_MEDIUM_FULL_HEIGHT: u8 = 16;\n");
    content.push_str("pub const FONT_MEDIUM_FULL_BYTES_PER_ROW: u8 = 2; // (16 + 7) / 8\n");
    content.push_str("pub const FONT_MEDIUM_FULL_GLYPH_SIZE: usize = 32; // 2 * 16\n\n");

    content.push_str("pub const FONT_MEDIUM_HALF_WIDTH: u8 = 8;\n");
    content.push_str("pub const FONT_MEDIUM_HALF_HEIGHT: u8 = 16;\n");
    content.push_str("pub const FONT_MEDIUM_HALF_BYTES_PER_ROW: u8 = 1; // (8 + 7) / 8\n");
    content.push_str("pub const FONT_MEDIUM_HALF_GLYPH_SIZE: usize = 16; // 1 * 16\n\n");

    // 大号字体常量
    content.push_str("// 大号字体 (24px)\n");
    content.push_str("pub const FONT_LARGE_FULL_WIDTH: u8 = 24;\n");
    content.push_str("pub const FONT_LARGE_FULL_HEIGHT: u8 = 24;\n");
    content.push_str("pub const FONT_LARGE_FULL_BYTES_PER_ROW: u8 = 3; // (24 + 7) / 8\n");
    content.push_str("pub const FONT_LARGE_FULL_GLYPH_SIZE: usize = 72; // 3 * 24\n\n");

    content.push_str("pub const FONT_LARGE_HALF_WIDTH: u8 = 12;\n");
    content.push_str("pub const FONT_LARGE_HALF_HEIGHT: u8 = 24;\n");
    content.push_str("pub const FONT_LARGE_HALF_BYTES_PER_ROW: u8 = 2; // (12 + 7) / 8\n");
    content.push_str("pub const FONT_LARGE_HALF_GLYPH_SIZE: usize = 48; // 2 * 24\n\n");

    // ========== 3. 位图数据 ==========
    content.push_str("// ==================== 位图数据 ====================\n\n");

    // 使用 include_bytes! 嵌入二进制数据
    content.push_str("// 小号字体位图\n");
    content.push_str("pub const SMALL_FULL_WIDTH_BITMAP: &[u8] = include_bytes!(\"generated_small_full_width_font.bin\");\n");
    content.push_str("pub const SMALL_HALF_WIDTH_BITMAP: &[u8] = include_bytes!(\"generated_small_half_width_font.bin\");\n\n");

    content.push_str("// 中号字体位图\n");
    content.push_str("pub const MEDIUM_FULL_WIDTH_BITMAP: &[u8] = include_bytes!(\"generated_medium_full_width_font.bin\");\n");
    content.push_str("pub const MEDIUM_HALF_WIDTH_BITMAP: &[u8] = include_bytes!(\"generated_medium_half_width_font.bin\");\n\n");

    content.push_str("// 大号字体位图\n");
    content.push_str("pub const LARGE_FULL_WIDTH_BITMAP: &[u8] = include_bytes!(\"generated_large_full_width_font.bin\");\n");
    content.push_str("pub const LARGE_HALF_WIDTH_BITMAP: &[u8] = include_bytes!(\"generated_large_half_width_font.bin\");\n\n");

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
    content.push_str("    Small,\n");
    content.push_str("    Medium,\n");
    content.push_str("    Large,\n");
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
    content.push_str("                FontSize::Small => {\n");
    content.push_str("                    let start = idx * FONT_SMALL_FULL_GLYPH_SIZE;\n");
    content.push_str("                    Some(&SMALL_FULL_WIDTH_BITMAP[start..start + FONT_SMALL_FULL_GLYPH_SIZE])\n");
    content.push_str("                }\n");
    content.push_str("                FontSize::Medium => {\n");
    content.push_str("                    let start = idx * FONT_MEDIUM_FULL_GLYPH_SIZE;\n");
    content.push_str("                    Some(&MEDIUM_FULL_WIDTH_BITMAP[start..start + FONT_MEDIUM_FULL_GLYPH_SIZE])\n");
    content.push_str("                }\n");
    content.push_str("                FontSize::Large => {\n");
    content.push_str("                    let start = idx * FONT_LARGE_FULL_GLYPH_SIZE;\n");
    content.push_str("                    Some(&LARGE_FULL_WIDTH_BITMAP[start..start + FONT_LARGE_FULL_GLYPH_SIZE])\n");
    content.push_str("                }\n");
    content.push_str("            }\n");
    content.push_str("        }\n");
    content.push_str("        CharWidth::Half => {\n");
    content.push_str("            let idx = find_char_index(HALF_WIDTH_CHARS, target)?;\n");
    content.push_str("            match size {\n");
    content.push_str("                FontSize::Small => {\n");
    content.push_str("                    let start = idx * FONT_SMALL_HALF_GLYPH_SIZE;\n");
    content.push_str("                    Some(&SMALL_HALF_WIDTH_BITMAP[start..start + FONT_SMALL_HALF_GLYPH_SIZE])\n");
    content.push_str("                }\n");
    content.push_str("                FontSize::Medium => {\n");
    content.push_str("                    let start = idx * FONT_MEDIUM_HALF_GLYPH_SIZE;\n");
    content.push_str("                    Some(&MEDIUM_HALF_WIDTH_BITMAP[start..start + FONT_MEDIUM_HALF_GLYPH_SIZE])\n");
    content.push_str("                }\n");
    content.push_str("                FontSize::Large => {\n");
    content.push_str("                    let start = idx * FONT_LARGE_HALF_GLYPH_SIZE;\n");
    content.push_str("                    Some(&LARGE_HALF_WIDTH_BITMAP[start..start + FONT_LARGE_HALF_GLYPH_SIZE])\n");
    content.push_str("                }\n");
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
    content.push_str("        (FontSize::Small, true) => (FONT_SMALL_FULL_WIDTH, FONT_SMALL_FULL_HEIGHT, FONT_SMALL_FULL_BYTES_PER_ROW),\n");
    content.push_str("        (FontSize::Small, false) => (FONT_SMALL_HALF_WIDTH, FONT_SMALL_HALF_HEIGHT, FONT_SMALL_HALF_BYTES_PER_ROW),\n");
    content.push_str("        (FontSize::Medium, true) => (FONT_MEDIUM_FULL_WIDTH, FONT_MEDIUM_FULL_HEIGHT, FONT_MEDIUM_FULL_BYTES_PER_ROW),\n");
    content.push_str("        (FontSize::Medium, false) => (FONT_MEDIUM_HALF_WIDTH, FONT_MEDIUM_HALF_HEIGHT, FONT_MEDIUM_HALF_BYTES_PER_ROW),\n");
    content.push_str("        (FontSize::Large, true) => (FONT_LARGE_FULL_WIDTH, FONT_LARGE_FULL_HEIGHT, FONT_LARGE_FULL_BYTES_PER_ROW),\n");
    content.push_str("        (FontSize::Large, false) => (FONT_LARGE_HALF_WIDTH, FONT_LARGE_HALF_HEIGHT, FONT_LARGE_HALF_BYTES_PER_ROW),\n");
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
