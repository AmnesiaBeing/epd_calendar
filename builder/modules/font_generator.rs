//! 字体生成模块 - 支持3种不同大小的字符集

use crate::builder::config::BuildConfig;
use crate::builder::utils::font_renderer::{FontConfig, FontRenderResult, FontRenderer};
use crate::builder::utils::{self, progress::ProgressTracker};
use anyhow::{Context, Result};
use std::fs;

/// 字体尺寸配置
#[derive(Debug, Clone)]
pub struct FontSizeConfig {
    pub name: &'static str,
    pub size: u32,
}

/// 支持的字体尺寸
const FONT_SIZES: [FontSizeConfig; 3] = [
    FontSizeConfig {
        name: "small",
        size: 12,
    },
    FontSizeConfig {
        name: "medium",
        size: 16,
    },
    FontSizeConfig {
        name: "large",
        size: 24,
    },
];

/// 构建字体数据
pub fn build(config: &BuildConfig, progress: &ProgressTracker) -> Result<()> {
    progress.update_progress(0, 4, "读取字符集");
    let chars = read_charset(config)?;

    progress.update_progress(1, 4, "分离全角和半角字符");
    let (full_width_chars, half_width_chars) = separate_chars(&chars);

    progress.update_progress(2, 4, "渲染字体");
    let font_results = render_fonts(config, &full_width_chars, &half_width_chars, progress)?;

    progress.update_progress(3, 4, "生成字体文件");
    generate_font_files(config, &font_results)?;

    println!(
        "cargo:warning=  字体生成完成，共处理字符: {}个（全角: {}个，半角: {}个）",
        chars.len(),
        full_width_chars.len(),
        half_width_chars.len()
    );

    Ok(())
}

/// 从文件读取字符集
fn read_charset(config: &BuildConfig) -> Result<Vec<char>> {
    let charset_path = config.font_path.parent().unwrap().join("chars.txt");

    let content = fs::read_to_string(&charset_path)
        .with_context(|| format!("读取字符集文件失败: {}", charset_path.display()))?;

    // 去重并过滤控制字符和空白字符
    let mut char_set = std::collections::BTreeSet::new();
    for c in content.chars() {
        if !c.is_control() && !c.is_whitespace() {
            char_set.insert(c);
        }
    }

    let chars: Vec<char> = char_set.into_iter().collect();
    println!(
        "cargo:warning=  从字符集文件中读取到 {} 个字符",
        chars.len()
    );

    Ok(chars)
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

    // 确保半角字符包含完整的ASCII可打印字符集
    let mut half_set: std::collections::BTreeSet<char> = half_width.into_iter().collect();
    for code in 0x20..=0x7E {
        if let Some(c) = std::char::from_u32(code) {
            if c.is_ascii_graphic() || c == ' ' {
                half_set.insert(c);
            }
        }
    }
    let half_width: Vec<char> = half_set.into_iter().collect();

    println!(
        "cargo:warning=  字符分离结果 - 全角: {}个, 半角: {}个",
        full_width.len(),
        half_width.len()
    );

    (full_width, half_width)
}

/// 检查是否为半角字符
fn is_half_width_char(c: char) -> bool {
    c.is_ascii() && c.is_ascii_graphic()
}

/// 渲染所有字体
fn render_fonts(
    config: &BuildConfig,
    full_width_chars: &[char],
    half_width_chars: &[char],
    progress: &ProgressTracker,
) -> Result<Vec<(FontSizeConfig, FontRenderResult, FontRenderResult)>> {
    let mut results = Vec::new();

    for (index, font_config) in FONT_SIZES.iter().enumerate() {
        progress.update_progress(
            index,
            FONT_SIZES.len(),
            &format!("渲染{}字体", font_config.name),
        );

        // 渲染全角字体
        let full_width_config = FontConfig {
            font_path: config.font_path.to_string_lossy().to_string(),
            font_size: font_config.size,
            is_half_width: false,
            chars: full_width_chars.to_vec(),
        };
        let full_width_result = FontRenderer::render_font(&full_width_config)?;

        // 渲染半角字体
        let half_width_config = FontConfig {
            font_path: config.font_path.to_string_lossy().to_string(),
            font_size: font_config.size,
            is_half_width: true,
            chars: half_width_chars.to_vec(),
        };
        let half_width_result = FontRenderer::render_font(&half_width_config)?;

        // 预览（在移动之前进行）
        FontRenderer::preview_string(
            &full_width_result,
            "你好，世界！",
            &format!("{}全角", font_config.name),
        );
        FontRenderer::preview_string(
            &half_width_result,
            "Hello World!",
            &format!("{}半角", font_config.name),
        );

        results.push((font_config.clone(), full_width_result, half_width_result));
    }

    Ok(results)
}

/// 生成字体文件
fn generate_font_files(
    config: &BuildConfig,
    font_results: &[(FontSizeConfig, FontRenderResult, FontRenderResult)],
) -> Result<()> {
    // 生成二进制字体文件
    for (font_config, full_width_result, half_width_result) in font_results {
        let full_width_bin_path = config.output_dir.join(format!(
            "generated_{}_full_width_font.bin",
            font_config.name
        ));
        utils::file_utils::write_file(&full_width_bin_path, &full_width_result.glyph_data)?;

        let half_width_bin_path = config.output_dir.join(format!(
            "generated_{}_half_width_font.bin",
            font_config.name
        ));
        utils::file_utils::write_file(&half_width_bin_path, &half_width_result.glyph_data)?;
    }

    // 生成字体描述文件
    let fonts_rs_path = config.output_dir.join("generated_fonts.rs");
    let content = generate_fonts_rs_content(font_results)?;
    utils::file_utils::write_string_file(&fonts_rs_path, &content)?;

    println!(
        "cargo:warning=  字体文件生成成功: {}",
        fonts_rs_path.display()
    );

    Ok(())
}

/// 生成字体描述文件内容
fn generate_fonts_rs_content(
    font_results: &[(FontSizeConfig, FontRenderResult, FontRenderResult)],
) -> Result<String> {
    let mut content = String::new();

    content.push_str("// 自动生成的字体描述文件\n");
    content.push_str("// 不要手动修改此文件\n\n");
    content.push_str("use embedded_graphics::{\n    image::ImageRaw,\n    mono_font::{DecorationDimensions, MonoFont, mapping::GlyphMapping},\n    pixelcolor::BinaryColor,\n    prelude::Size,\n};\n\n");

    content.push_str("struct BinarySearchGlyphMapping {\n    chars: &'static [u16],\n    offsets: &'static [u32],\n}\n\n");

    content.push_str("impl GlyphMapping for BinarySearchGlyphMapping {\n    fn index(&self, c: char) -> usize {\n        let target = c as u16;\n        let mut left = 0;\n        let mut right = self.chars.len();\n        \n        while left < right {\n            let mid = left + (right - left) / 2;\n            if self.chars[mid] < target {\n                left = mid + 1;\n            } else if self.chars[mid] > target {\n                right = mid;\n            } else {\n                return self.offsets[mid] as usize;\n            }\n        }\n        \n        0\n    }\n}\n\n");

    // 为每种字体尺寸生成映射
    for (font_config, full_width_result, half_width_result) in font_results {
        // 生成全角字符映射
        let full_width_offsets: Vec<u32> =
            full_width_result.char_mapping.values().cloned().collect();
        content.push_str(&format!(
            "const {}_FULL_WIDTH_CHARS: &[u16] = &[\n",
            font_config.name.to_uppercase()
        ));
        for (&c, _) in full_width_result.char_mapping.iter() {
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

        content.push_str(&format!(
            "const {}_FULL_WIDTH_OFFSETS: &[u32] = &[\n",
            font_config.name.to_uppercase()
        ));
        for &offset in &full_width_offsets {
            content.push_str(&format!("    {},\n", offset));
        }
        content.push_str("];\n\n");

        // 生成半角字符映射
        let half_width_offsets: Vec<u32> =
            half_width_result.char_mapping.values().cloned().collect();
        content.push_str(&format!(
            "const {}_HALF_WIDTH_CHARS: &[u16] = &[\n",
            font_config.name.to_uppercase()
        ));
        for (&c, _) in half_width_result.char_mapping.iter() {
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

        content.push_str(&format!(
            "const {}_HALF_WIDTH_OFFSETS: &[u32] = &[\n",
            font_config.name.to_uppercase()
        ));
        for &offset in &half_width_offsets {
            content.push_str(&format!("    {},\n", offset));
        }
        content.push_str("];\n\n");

        // 生成映射结构体
        content.push_str(&format!("static {}_FULL_WIDTH_GLYPH_MAPPING: BinarySearchGlyphMapping = BinarySearchGlyphMapping {{\n    chars: {}_FULL_WIDTH_CHARS,\n    offsets: {}_FULL_WIDTH_OFFSETS,\n}};\n\n", font_config.name.to_uppercase(), font_config.name.to_uppercase(), font_config.name.to_uppercase()));

        content.push_str(&format!("static {}_HALF_WIDTH_GLYPH_MAPPING: BinarySearchGlyphMapping = BinarySearchGlyphMapping {{\n    chars: {}_HALF_WIDTH_CHARS,\n    offsets: {}_HALF_WIDTH_OFFSETS,\n}};\n\n", font_config.name.to_uppercase(), font_config.name.to_uppercase(), font_config.name.to_uppercase()));

        // 生成字体定义
        content.push_str(&format!("pub const {}_FULL_WIDTH_FONT: MonoFont<'static> = MonoFont {{\n    image: ImageRaw::<BinaryColor>::new(
        include_bytes!(\"generated_{}_full_width_font.bin\"),\n        {}\n    ),\n    glyph_mapping: &{}_FULL_WIDTH_GLYPH_MAPPING,\n    character_size: Size::new({}, {}),\n    character_spacing: 0,\n    baseline: {},\n    underline: DecorationDimensions::new({} + 2, 1),\n    strikethrough: DecorationDimensions::new({} / 2, 1),\n}};\n\n",
            font_config.name.to_uppercase(),
            font_config.name,
            font_config.size,
            font_config.name.to_uppercase(),
            full_width_result.char_width,
            full_width_result.char_height,
            0,
            font_config.size,
            font_config.size
        ));

        content.push_str(&format!("pub const {}_HALF_WIDTH_FONT: MonoFont<'static> = MonoFont {{\n    image: ImageRaw::<BinaryColor>::new(
        include_bytes!(\"generated_{}_half_width_font.bin\"),\n        {}\n    ),\n    glyph_mapping: &{}_HALF_WIDTH_GLYPH_MAPPING,\n    character_size: Size::new({}, {}),\n    character_spacing: 0,\n    baseline: {},\n    underline: DecorationDimensions::new({} + 2, 1),\n    strikethrough: DecorationDimensions::new({} / 2, 1),\n}};\n\n",
            font_config.name.to_uppercase(),
            font_config.name,
            font_config.size,
            font_config.name.to_uppercase(),
            half_width_result.char_width,
            half_width_result.char_height,
            0,
            font_config.size,
            half_width_result.char_width
        ));
    }

    Ok(content)
}
