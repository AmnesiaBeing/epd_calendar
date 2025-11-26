//! 日期显示字体处理模块

use crate::builder::config::BuildConfig;
use crate::builder::utils::font_renderer::{FontConfig, FontRenderResult, FontRenderer};
use crate::builder::utils::{self, progress::ProgressTracker};
use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;

const FONT_SIZE: u32 = 40;

/// 构建字体数据
pub fn build(config: &BuildConfig, progress: &ProgressTracker) -> Result<()> {
    progress.update_progress(2, 5, "收集字符数据");
    let all_chars = collect_all_chars()?;

    progress.update_progress(4, 5, "渲染日期字体");
    generate_date_fonts(config, &all_chars, progress)?;

    Ok(())
}

/// 收集所有格言中使用的字符
fn collect_all_chars() -> Result<Vec<char>> {
    let filtered_chars: Vec<char> = vec![
        '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', '-', '月', '日', '年', '时', '分', '秒',
        '一', '二', '三', '四', '五', '六', '七', '八', '九', '十', '〇', '周', '工', '休',
    ];

    println!(
        "cargo:warning=  收集到日期相关字符: {}个",
        filtered_chars.len()
    );

    Ok(filtered_chars)
}

/// 检查是否为半角字符
fn is_half_width_char(c: char) -> bool {
    c.is_ascii() && c.is_ascii_graphic()
}

/// 分离全角和半角字符
fn separate_chars(chars: &[char]) -> (Vec<char>, Vec<char>) {
    let mut full_width = Vec::new();
    let mut half_width = Vec::new();

    for &c in chars {
        if is_half_width_char(c) {
            half_width.push(c);
        } else {
            full_width.push(c);
        }
    }

    println!(
        "cargo:warning=  字体字符分离 - 全角: {}个, 半角: {}个",
        full_width.len(),
        half_width.len()
    );

    (full_width, half_width)
}

/// 生成日期字体文件
fn generate_date_fonts(
    config: &BuildConfig,
    chars: &[char],
    progress: &ProgressTracker,
) -> Result<()> {
    // 分离全角和半角字符
    let (full_width_chars, half_width_chars) = separate_chars(chars);

    // 渲染全角字体（使用40px大小）
    let full_width_config = FontConfig {
        font_path: config.font_path.to_string_lossy().to_string(),
        font_size: 40, // 日期显示使用较大的字体
        is_half_width: false,
        chars: full_width_chars,
    };

    progress.update_progress(0, 3, "渲染全角字体");
    let full_width_result = FontRenderer::render_font(&full_width_config)?;

    // 渲染半角字体
    let half_width_config = FontConfig {
        font_path: config.font_path.to_string_lossy().to_string(),
        font_size: 40,
        is_half_width: true,
        chars: half_width_chars,
    };

    progress.update_progress(1, 3, "渲染半角字体");
    let half_width_result = FontRenderer::render_font(&half_width_config)?;

    // 生成字体文件
    progress.update_progress(2, 3, "生成字体文件");
    generate_font_files(config, &full_width_result, &half_width_result)?;

    // 预览
    FontRenderer::preview_string(&full_width_result, "周一", "日期全角");
    FontRenderer::preview_string(&half_width_result, "123-456-789", "日期半角");

    Ok(())
}

/// 生成字体文件
fn generate_font_files(
    config: &BuildConfig,
    full_width_result: &FontRenderResult,
    half_width_result: &FontRenderResult,
) -> Result<()> {
    // 生成二进制字体文件
    let full_width_bin_path = config.output_dir.join("generated_date_full_width_font.bin");
    utils::file_utils::write_file(&full_width_bin_path, &full_width_result.glyph_data)?;

    let half_width_bin_path = config.output_dir.join("generated_date_half_width_font.bin");
    utils::file_utils::write_file(&half_width_bin_path, &half_width_result.glyph_data)?;

    // 生成字体描述文件
    let fonts_rs_path = config.output_dir.join("generated_date_fonts.rs");
    let content = generate_fonts_rs_content(
        &full_width_result.char_mapping,
        &half_width_result.char_mapping,
    )?;

    utils::file_utils::write_string_file(&fonts_rs_path, &content)?;

    println!(
        "cargo:warning=  日期字体文件生成成功: {}",
        fonts_rs_path.display()
    );

    Ok(())
}

/// 生成字体描述文件内容
fn generate_fonts_rs_content(
    full_width_char_mapping: &BTreeMap<char, u32>,
    half_width_char_mapping: &BTreeMap<char, u32>,
) -> Result<String> {
    let mut content = String::new();

    content.push_str("// 自动生成的格言字体描述文件\n");
    content.push_str("// 不要手动修改此文件\n\n");
    content.push_str("use embedded_graphics::{\n    image::ImageRaw,\n    mono_font::{DecorationDimensions, MonoFont, mapping::GlyphMapping},\n    pixelcolor::BinaryColor,\n    prelude::Size,\n};\n\n");

    content.push_str("pub struct BinarySearchGlyphMapping {\n    chars: &'static [u16],\n    offsets: &'static [u32],\n}\n\n");

    content.push_str("impl GlyphMapping for BinarySearchGlyphMapping {\n    fn index(&self, c: char) -> usize {\n        let target = c as u16;\n        let mut left = 0;\n        let mut right = self.chars.len();\n        \n        while left < right {\n            let mid = left + (right - left) / 2;\n            if self.chars[mid] < target {\n                left = mid + 1;\n            } else if self.chars[mid] > target {\n                right = mid;\n            } else {\n                return self.offsets[mid] as usize;\n            }\n        }\n        \n        0\n    }\n}\n\n");

    // 生成全角字符映射
    let full_width_offsets: Vec<u32> = full_width_char_mapping.values().cloned().collect();
    content.push_str("const DATE_FULL_WIDTH_CHARS: &[u16] = &[\n");
    for (&c, _) in full_width_char_mapping.iter() {
        let char_display = match c {
            '\n' => "\\n".to_string(),
            '\r' => "\\r".to_string(),
            '\t' => "\\t".to_string(),
            '\\' => "\\\\".to_string(),
            '"' => "\\\"".to_string(),
            _ if c.is_control() => format!("\\u{:04X}", c as u32),
            _ => c.to_string(),
        };
        content.push_str(&format!(
            "    {}, // '{}' (U+{:04X})\n",
            c as u16, char_display, c as u32
        ));
    }
    content.push_str("];\n\n");

    content.push_str("const DATE_FULL_WIDTH_OFFSETS: &[u32] = &[\n");
    for &offset in &full_width_offsets {
        content.push_str(&format!("    {},\n", offset));
    }
    content.push_str("];\n\n");

    // 生成半角字符映射
    let half_width_offsets: Vec<u32> = half_width_char_mapping.values().cloned().collect();
    content.push_str("const DATE_HALF_WIDTH_CHARS: &[u16] = &[\n");
    for (&c, _) in half_width_char_mapping.iter() {
        let char_display = match c {
            '\n' => "\\n".to_string(),
            '\r' => "\\r".to_string(),
            '\t' => "\\t".to_string(),
            '\\' => "\\\\".to_string(),
            '"' => "\\\"".to_string(),
            _ if c.is_control() => format!("\\u{:04X}", c as u32),
            _ => c.to_string(),
        };
        content.push_str(&format!(
            "    {}, // '{}' (U+{:04X})\n",
            c as u16, char_display, c as u32
        ));
    }
    content.push_str("];\n\n");

    content.push_str("const DATE_HALF_WIDTH_OFFSETS: &[u32] = &[\n");
    for &offset in &half_width_offsets {
        content.push_str(&format!("    {},\n", offset));
    }
    content.push_str("];\n\n");

    content.push_str("pub static DATE_FULL_WIDTH_GLYPH_MAPPING: BinarySearchGlyphMapping = BinarySearchGlyphMapping {\n    chars: DATE_FULL_WIDTH_CHARS,\n    offsets: DATE_FULL_WIDTH_OFFSETS,\n};\n\n");

    content.push_str("pub static DATE_HALF_WIDTH_GLYPH_MAPPING: BinarySearchGlyphMapping = BinarySearchGlyphMapping {\n    chars: DATE_HALF_WIDTH_CHARS,\n    offsets: DATE_HALF_WIDTH_OFFSETS,\n};\n\n");

    // // 生成全角字体定义
    // content.push_str(&format!(
    //     "pub const DATE_FULL_WIDTH_FONT: MonoFont<'static> = MonoFont {{\n    image: ImageRaw::<BinaryColor>::new(\n        include_bytes!(\"generated_date_full_width_font.bin\"),\n        {}\n    ),\n    glyph_mapping: &DATE_FULL_WIDTH_GLYPH_MAPPING,\n    character_size: Size::new({}, {}),\n    character_spacing: 0,\n    baseline: {},\n    underline: DecorationDimensions::new({} + 2, 1),\n    strikethrough: DecorationDimensions::new({} / 2, 1),\n}};\n\n",
    //     FONT_SIZE, full_width, full_height, 0, FONT_SIZE, FONT_SIZE
    // ));

    // // 生成半角字体定义
    // content.push_str(&format!(
    //     "pub const DATE_HALF_WIDTH_FONT: MonoFont<'static> = MonoFont {{\n    image: ImageRaw::<BinaryColor>::new(\n        include_bytes!(\"generated_date_half_width_font.bin\"),\n        {}\n    ),\n    glyph_mapping: &DATE_HALF_WIDTH_GLYPH_MAPPING,\n    character_size: Size::new({}, {}),\n    character_spacing: 0,\n    baseline: {},\n    underline: DecorationDimensions::new({} + 2, 1),\n    strikethrough: DecorationDimensions::new({} / 2, 1),\n}};\n",
    //     FONT_SIZE, half_width, half_height, 0, FONT_SIZE, half_width
    // ));

    Ok(content)
}
