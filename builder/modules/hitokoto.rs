//! 格言数据处理模块

use crate::builder::config::BuildConfig;
use crate::builder::utils::font_renderer::{FontConfig, FontRenderResult, FontRenderer};
use crate::builder::utils::{self, ProgressTracker};
use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use unicode_normalization::UnicodeNormalization;

#[derive(Debug, Deserialize, Clone)]
pub struct Hitokoto {
    pub hitokoto: String,
    pub from: String,
    pub from_who: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct HitokotoCategory {
    pub id: u32,
    #[allow(dead_code)]
    pub name: String,
    pub key: String,
}

/// 构建格言数据
pub fn build(config: &BuildConfig, progress: &ProgressTracker) -> Result<()> {
    progress.update_progress(0, 5, "解析分类");
    let categories = parse_categories(config)?;

    progress.update_progress(1, 5, "解析格言文件");
    let hitokotos = parse_all_json_files(config, &categories)?;

    progress.update_progress(2, 5, "收集字符数据");
    let all_chars = collect_all_chars(&hitokotos)?;

    progress.update_progress(3, 5, "生成数据文件");
    generate_hitokoto_data(config, &hitokotos)?;

    progress.update_progress(4, 5, "渲染格言字体");
    generate_hitokoto_fonts(config, &all_chars, progress)?;

    Ok(())
}

pub fn parse_categories(config: &BuildConfig) -> Result<Vec<HitokotoCategory>> {
    let content = fs::read_to_string(&config.categories_path)
        .with_context(|| format!("读取分类文件失败: {}", config.categories_path.display()))?;
    serde_json::from_str(&content).context("解析categories.json失败")
}

pub fn parse_all_json_files(
    config: &BuildConfig,
    categories: &[HitokotoCategory],
) -> Result<Vec<(u32, Vec<Hitokoto>)>> {
    let mut result = Vec::new();

    for (index, category) in categories.iter().enumerate() {
        let mut path = config.sentences_dir.join(&category.key);
        path.set_extension("json");

        let content = fs::read_to_string(&path)
            .with_context(|| format!("读取JSON文件失败: {}", path.display()))?;

        let hitokotos: Vec<Hitokoto> = serde_json::from_str(&content)
            .with_context(|| format!("解析JSON失败: {}", path.display()))?;

        result.push((category.id, hitokotos));

        // 更新进度
        println!(
            "cargo:warning=  已处理分类: {}/{}",
            index + 1,
            categories.len()
        );
    }

    Ok(result)
}

/// 收集所有格言中使用的字符
fn collect_all_chars(hitokotos: &[(u32, Vec<Hitokoto>)]) -> Result<Vec<char>> {
    let mut char_set = BTreeSet::new();

    for (_, hitokoto_list) in hitokotos {
        for hitokoto in hitokoto_list {
            // 处理格言内容
            for c in hitokoto.hitokoto.nfc() {
                if !c.is_control() && !c.is_whitespace() {
                    char_set.insert(c);
                }
            }
            // 处理来源
            for c in hitokoto.from.nfc() {
                if !c.is_control() && !c.is_whitespace() {
                    char_set.insert(c);
                }
            }
            // 处理作者
            for c in hitokoto
                .from_who
                .clone()
                .unwrap_or_else(|| "佚名".to_string())
                .nfc()
            {
                if !c.is_control() && !c.is_whitespace() {
                    char_set.insert(c);
                }
            }
        }
    }

    let filtered_chars: Vec<char> = char_set.into_iter().collect();

    println!(
        "cargo:warning=  收集到格言相关字符: {}个",
        filtered_chars.len()
    );

    // 输出字符统计信息
    let (full_width_count, half_width_count) = count_full_half_width_chars(&filtered_chars);
    println!(
        "cargo:warning=  字符分类 - 全角: {}个, 半角: {}个",
        full_width_count, half_width_count
    );

    Ok(filtered_chars)
}

/// 统计全角和半角字符数量
fn count_full_half_width_chars(chars: &[char]) -> (usize, usize) {
    let mut full_width = 0;
    let mut half_width = 0;

    for &c in chars {
        if is_half_width_char(c) {
            half_width += 1;
        } else {
            full_width += 1;
        }
    }

    (full_width, half_width)
}

/// 检查是否为半角字符
fn is_half_width_char(c: char) -> bool {
    c.is_ascii() && c.is_ascii_graphic()
}

/// 生成格言字体文件
fn generate_hitokoto_fonts(
    config: &BuildConfig,
    chars: &[char],
    progress: &ProgressTracker,
) -> Result<()> {
    // 分离全角和半角字符
    let (full_width_chars, half_width_chars) = separate_chars(chars);

    // 渲染全角字体（使用24px大小）
    let full_width_config = FontConfig {
        font_path: config.font_path.to_string_lossy().to_string(),
        font_size: 24, // 格言显示使用较大的字体
        is_half_width: false,
        chars: full_width_chars,
    };

    progress.update_progress(0, 3, "渲染全角字体");
    let full_width_result = FontRenderer::render_font(&full_width_config)?;

    // 渲染半角字体
    let half_width_config = FontConfig {
        font_path: config.font_path.to_string_lossy().to_string(),
        font_size: 24,
        is_half_width: true,
        chars: half_width_chars,
    };

    progress.update_progress(1, 3, "渲染半角字体");
    let half_width_result = FontRenderer::render_font(&half_width_config)?;

    // 生成字体文件
    progress.update_progress(2, 3, "生成字体文件");
    generate_font_files(config, &full_width_result, &half_width_result)?;

    // 预览
    FontRenderer::preview_string(&full_width_result, "你好，世界！", "格言全角");
    FontRenderer::preview_string(&half_width_result, "Hello World!", "格言半角");

    Ok(())
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
    let mut half_set: BTreeSet<char> = half_width.into_iter().collect();
    for code in 0x20..=0x7E {
        if let Some(c) = std::char::from_u32(code) {
            if c.is_ascii_graphic() || c == ' ' {
                half_set.insert(c);
            }
        }
    }
    let half_width: Vec<char> = half_set.into_iter().collect();

    println!(
        "cargo:warning=  字体字符分离 - 全角: {}个, 半角: {}个",
        full_width.len(),
        half_width.len()
    );

    (full_width, half_width)
}

/// 生成字体文件
fn generate_font_files(
    config: &BuildConfig,
    full_width_result: &FontRenderResult,
    half_width_result: &FontRenderResult,
) -> Result<()> {
    // 生成二进制字体文件
    let full_width_bin_path = config.output_dir.join("hitokoto_full_width_font.bin");
    utils::file_utils::write_file(&full_width_bin_path, &full_width_result.glyph_data)?;

    let half_width_bin_path = config.output_dir.join("hitokoto_half_width_font.bin");
    utils::file_utils::write_file(&half_width_bin_path, &half_width_result.glyph_data)?;

    // 生成字体描述文件
    let fonts_rs_path = config.output_dir.join("hitokoto_fonts.rs");
    let content = generate_fonts_rs_content(
        &full_width_result.char_mapping,
        &half_width_result.char_mapping,
        full_width_result.char_width,
        full_width_result.char_height,
        half_width_result.char_width,
        half_width_result.char_height,
    )?;

    utils::file_utils::write_string_file(&fonts_rs_path, &content)?;

    println!(
        "cargo:warning=  格言字体文件生成成功: {}",
        fonts_rs_path.display()
    );

    Ok(())
}

/// 生成字体描述文件内容
fn generate_fonts_rs_content(
    full_width_char_mapping: &BTreeMap<char, u32>,
    half_width_char_mapping: &BTreeMap<char, u32>,
    full_width: u32,
    full_height: u32,
    half_width: u32,
    half_height: u32,
) -> Result<String> {
    let mut content = String::new();

    content.push_str("// 自动生成的格言字体描述文件\n");
    content.push_str("// 不要手动修改此文件\n\n");
    content.push_str("use embedded_graphics::{\n    image::ImageRaw,\n    mono_font::{DecorationDimensions, MonoFont, mapping::GlyphMapping},\n    pixelcolor::BinaryColor,\n    prelude::Size,\n};\n\n");

    content.push_str("struct BinarySearchGlyphMapping {\n    chars: &'static [u16],\n    offsets: &'static [u32],\n}\n\n");

    content.push_str("impl GlyphMapping for BinarySearchGlyphMapping {\n    fn index(&self, c: char) -> usize {\n        let target = c as u16;\n        let mut left = 0;\n        let mut right = self.chars.len();\n        \n        while left < right {\n            let mid = left + (right - left) / 2;\n            if self.chars[mid] < target {\n                left = mid + 1;\n            } else if self.chars[mid] > target {\n                right = mid;\n            } else {\n                return self.offsets[mid] as usize;\n            }\n        }\n        \n        0\n    }\n}\n\n");

    // 生成全角字符映射
    let full_width_offsets: Vec<u32> = full_width_char_mapping.values().cloned().collect();
    content.push_str("const HITOKOTO_FULL_WIDTH_CHARS: &[u16] = &[\n");
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

    content.push_str("const HITOKOTO_FULL_WIDTH_OFFSETS: &[u32] = &[\n");
    for &offset in &full_width_offsets {
        content.push_str(&format!("    {},\n", offset));
    }
    content.push_str("];\n\n");

    // 生成半角字符映射
    let half_width_offsets: Vec<u32> = half_width_char_mapping.values().cloned().collect();
    content.push_str("const HITOKOTO_HALF_WIDTH_CHARS: &[u16] = &[\n");
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

    content.push_str("const HITOKOTO_HALF_WIDTH_OFFSETS: &[u32] = &[\n");
    for &offset in &half_width_offsets {
        content.push_str(&format!("    {},\n", offset));
    }
    content.push_str("];\n\n");

    content.push_str("static HITOKOTO_FULL_WIDTH_GLYPH_MAPPING: BinarySearchGlyphMapping = BinarySearchGlyphMapping {\n    chars: HITOKOTO_FULL_WIDTH_CHARS,\n    offsets: HITOKOTO_FULL_WIDTH_OFFSETS,\n};\n\n");

    content.push_str("static HITOKOTO_HALF_WIDTH_GLYPH_MAPPING: BinarySearchGlyphMapping = BinarySearchGlyphMapping {\n    chars: HITOKOTO_HALF_WIDTH_CHARS,\n    offsets: HITOKOTO_HALF_WIDTH_OFFSETS,\n};\n\n");

    // 生成全角字体定义
    content.push_str(&format!(
        "pub const HITOKOTO_FULL_WIDTH_FONT: MonoFont<'static> = MonoFont {{\n    image: ImageRaw::<BinaryColor>::new(\n        include_bytes!(\"hitokoto_full_width_font.bin\"),\n        {}\n    ),\n    glyph_mapping: &HITOKOTO_FULL_WIDTH_GLYPH_MAPPING,\n    character_size: Size::new({}, {}),\n    character_spacing: 0,\n    baseline: {},\n    underline: DecorationDimensions::new({} + 2, 1),\n    strikethrough: DecorationDimensions::new({} / 2, 1),\n}};\n\n",
        24, full_width, full_height, 24, 24, 24
    ));

    // 生成半角字体定义
    content.push_str(&format!(
        "pub const HITOKOTO_HALF_WIDTH_FONT: MonoFont<'static> = MonoFont {{\n    image: ImageRaw::<BinaryColor>::new(\n        include_bytes!(\"hitokoto_half_width_font.bin\"),\n        {}\n    ),\n    glyph_mapping: &HITOKOTO_HALF_WIDTH_GLYPH_MAPPING,\n    character_size: Size::new({}, {}),\n    character_spacing: 0,\n    baseline: {},\n    underline: DecorationDimensions::new({} + 2, 1),\n    strikethrough: DecorationDimensions::new({} / 2, 1),\n}};\n",
        24, half_width, half_height, 24, 24, half_width
    ));

    Ok(content)
}

fn generate_hitokoto_data(config: &BuildConfig, hitokotos: &[(u32, Vec<Hitokoto>)]) -> Result<()> {
    let output_path = config.output_dir.join("hitokoto_data.rs");

    let mut from_strings = BTreeSet::new();
    let mut from_who_strings = BTreeSet::new();
    let mut all_hitokotos = Vec::new();

    from_who_strings.insert("佚名".to_string());

    for (category_id, hitokoto_list) in hitokotos {
        for hitokoto in hitokoto_list {
            from_strings.insert(hitokoto.from.clone());
            if let Some(from_who) = &hitokoto.from_who {
                from_who_strings.insert(from_who.clone());
            }
            all_hitokotos.push((*category_id, hitokoto));
        }
    }

    let from_vec: Vec<String> = from_strings.into_iter().collect();
    let from_who_vec: Vec<String> = from_who_strings.into_iter().collect();

    let from_index_map: HashMap<&str, usize> = from_vec
        .iter()
        .enumerate()
        .map(|(i, s)| (s.as_str(), i))
        .collect();

    let from_who_index_map: HashMap<&str, usize> = from_who_vec
        .iter()
        .enumerate()
        .map(|(i, s)| (s.as_str(), i))
        .collect();

    let yiming_index = from_who_index_map["佚名"];

    let mut content = String::new();
    content.push_str("// 自动生成的格言数据文件\n// 不要手动修改此文件\n\n");

    // 生成 FROM_STRINGS 数组
    content.push_str("pub const FROM_STRINGS: &[&str] = &[\n");
    for from_str in &from_vec {
        content.push_str(&format!(
            "    \"{}\",\n",
            utils::string_utils::escape_string(from_str)
        ));
    }
    content.push_str("];\n\n");

    // 生成 FROM_WHO_STRINGS 数组
    content.push_str("pub const FROM_WHO_STRINGS: &[&str] = &[\n");
    for from_who_str in &from_who_vec {
        content.push_str(&format!(
            "    \"{}\",\n",
            utils::string_utils::escape_string(from_who_str)
        ));
    }
    content.push_str("];\n\n");

    // 生成 Hitokoto 结构体和数据数组
    content.push_str("#[derive(Debug, Clone, Copy)]\npub struct Hitokoto {\n    pub hitokoto: &'static str,\n    pub from: usize,\n    pub from_who: usize,\n    pub category: u32,\n}\n\n");
    content.push_str("pub const HITOKOTOS: &[Hitokoto] = &[\n");

    for (category_id, hitokoto) in &all_hitokotos {
        let from_index = from_index_map[hitokoto.from.as_str()];
        let from_who_index = if let Some(from_who) = &hitokoto.from_who {
            from_who_index_map[from_who.as_str()]
        } else {
            yiming_index
        };

        content.push_str("    Hitokoto {\n");
        content.push_str(&format!(
            "        hitokoto: \"{}\",\n",
            utils::string_utils::escape_string(&hitokoto.hitokoto)
        ));
        content.push_str(&format!("        from: {},\n", from_index));
        content.push_str(&format!("        from_who: {},\n", from_who_index));
        content.push_str(&format!("        category: {},\n", category_id));
        content.push_str("    },\n");
    }
    content.push_str("];\n");

    utils::file_utils::write_string_file(&output_path, &content)?;

    println!(
        "cargo:warning=  生成格言数据: 来源{}个, 作者{}个, 格言{}条",
        from_vec.len(),
        from_who_vec.len(),
        all_hitokotos.len()
    );

    Ok(())
}
