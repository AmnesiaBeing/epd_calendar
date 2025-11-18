//! 字体渲染和生成模块

use crate::builder::config::BuildConfig;
use crate::builder::utils::{self, ProgressTracker};
use anyhow::{Result, anyhow};
use freetype::Library;
use freetype::bitmap::PixelMode;
use std::collections::BTreeMap;

/// 构建字体数据
pub fn build(config: &BuildConfig, progress: &ProgressTracker) -> Result<()> {
    progress.update_progress(0, 5, "收集CJK字符数据");
    let all_chars = collect_ascii_cjk_chars()?;

    progress.update_progress(1, 5, "分离全角半角字符");
    let (full_width_chars, half_width_chars) = separate_chars(&all_chars);

    progress.update_progress(2, 5, "渲染全角字体");
    let (full_width_glyph_data, full_width_char_mapping) =
        render_full_width_font(config, &full_width_chars, progress)?;

    progress.update_progress(3, 5, "渲染半角字体");
    let (half_width_glyph_data, half_width_char_mapping) =
        render_half_width_font(config, &half_width_chars, progress)?;

    progress.update_progress(4, 5, "生成字体文件");
    generate_font_files(
        config,
        &full_width_glyph_data,
        &full_width_char_mapping,
        &half_width_glyph_data,
        &half_width_char_mapping,
    )?;

    Ok(())
}

/// 收集所有 CJK 字符 (Unicode 0x0000-0xFFFF 范围内)
fn collect_ascii_cjk_chars() -> Result<Vec<char>> {
    let mut ret = Vec::new();

    // CJK 统一汉字范围
    let cjk_ranges = [
        (0x4E00, 0x9FFF),   // CJK 统一汉字
        (0x3400, 0x4DBF),   // CJK 统一汉字扩展A
        (0x20000, 0x2A6DF), // CJK 统一汉字扩展B
        (0xF900, 0xFAFF),   // CJK 兼容汉字
        (0x2E80, 0x2EFF),   // CJK 部首补充
        (0x3000, 0x303F),   // CJK 符号和标点
        (0x31C0, 0x31EF),   // CJK 笔画
        (0x3200, 0x32FF),   // 带圈CJK字符和月份
        (0x3300, 0x33FF),   // CJK 兼容
        (0xFF00, 0xFFEF),   // 半角及全角形式
    ];

    // 收集 CJK 字符
    for (start, end) in cjk_ranges.iter() {
        // 只处理在 BMP (0x0000-0xFFFF) 范围内的字符
        if *end <= 0xFFFF {
            for code_point in *start..=*end {
                if let Some(c) = std::char::from_u32(code_point) {
                    // 使用标准库函数过滤不可见和控制字符
                    if !c.is_control() && !c.is_whitespace() && c != '\0' {
                        ret.push(c);
                    }
                }
            }
        }
    }

    // 添加基本的 ASCII 可打印字符
    for code_point in 0x0020..=0x007E {
        if let Some(c) = std::char::from_u32(code_point) {
            if !c.is_control() {
                ret.push(c);
            }
        }
    }

    println!("cargo:warning=  收集到 ASCII 和 CJK 可用字符：{}个", ret.len());
    Ok(ret)
}

/// 检查是否为半角字符
fn is_half_width_char(c: char) -> bool {
    // ASCII 可打印字符视为半角
    c.is_ascii_graphic() || c == ' '
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

    // 确保包含基本的 ASCII 字符集
    let mut half_set: std::collections::BTreeSet<char> = half_width.into_iter().collect();
    for code in 0x20..=0x7E {
        if let Some(c) = std::char::from_u32(code) {
            if !c.is_control() {
                half_set.insert(c);
            }
        }
    }
    let half_width: Vec<char> = half_set.into_iter().collect();

    println!(
        "cargo:warning=  全角字符：{}个，半角字符：{}个",
        full_width.len(),
        half_width.len()
    );

    (full_width, half_width)
}

/// 渲染全角字体
fn render_full_width_font(
    config: &BuildConfig,
    chars: &[char],
    progress: &ProgressTracker,
) -> Result<(Vec<u8>, BTreeMap<char, u32>)> {
    let lib = Library::init().map_err(|e| anyhow!("初始化freetype库失败: {}", e))?;
    let face = lib
        .new_face(&config.font_path, 0)
        .map_err(|e| anyhow!("加载字体文件失败 '{}': {}", config.font_path.display(), e))?;

    face.set_pixel_sizes(0, config.font_size)
        .map_err(|e| anyhow!("设置字体大小失败: {}", e))?;

    let mut result = Vec::new();
    let mut char_mapping = BTreeMap::new();

    let bytes_per_row = (config.font_size + 7) / 8;
    let char_data_size = (bytes_per_row * config.font_size) as usize;

    for (index, &c) in chars.iter().enumerate() {
        let Some(glyph_index) = face.get_char_index(c as usize) else {
            // 对于 CJK 字符，如果字体不支持，跳过即可
            continue;
        };

        if glyph_index == 0 {
            continue;
        }

        face.load_glyph(glyph_index, freetype::face::LoadFlag::RENDER)
            .map_err(|e| anyhow!("加载字形失败 '{}': {}", c, e))?;

        let glyph = face.glyph();
        let bitmap = glyph.bitmap();

        let bitmap_width = bitmap.width();
        let bitmap_rows = bitmap.rows();
        let bitmap_pitch = bitmap.pitch();

        char_mapping.insert(c, (result.len() / char_data_size) as u32);

        let current_len = result.len();
        result.resize(current_len + char_data_size, 0);

        let char_data = &mut result[current_len..current_len + char_data_size];
        char_data.fill(0);

        let x_offset = if (bitmap_width as u32) < config.font_size {
            (config.font_size - bitmap_width as u32) / 2
        } else {
            0
        };

        let y_offset = if (bitmap_rows as u32) < config.font_size {
            (config.font_size - bitmap_rows as u32) / 2
        } else {
            0
        };

        for y in 0..(bitmap_rows as u32) {
            let target_y = y + y_offset;
            if target_y >= config.font_size {
                break;
            }

            for x in 0..(bitmap_width as u32) {
                let target_x = x + x_offset;
                if target_x >= config.font_size {
                    break;
                }

                let src_x = x as i32;
                let src_y = y as i32;

                let pixel_value = match bitmap.pixel_mode() {
                    Ok(PixelMode::Mono) => {
                        let byte_index = (src_y * bitmap_pitch.abs() + src_x / 8) as usize;
                        if byte_index < bitmap.buffer().len() {
                            let byte = bitmap.buffer()[byte_index];
                            let bit_index = 7 - (src_x % 8);
                            (byte >> bit_index) & 1
                        } else {
                            0
                        }
                    }
                    Ok(PixelMode::Gray) => {
                        let pixel_index = (src_y * bitmap_pitch.abs() + src_x) as usize;
                        if pixel_index < bitmap.buffer().len() {
                            bitmap.buffer()[pixel_index]
                        } else {
                            0
                        }
                    }
                    _ => 0,
                };

                let is_black = match bitmap.pixel_mode() {
                    Ok(PixelMode::Mono) => pixel_value == 1,
                    Ok(PixelMode::Gray) => pixel_value > 128,
                    _ => false,
                };

                if is_black {
                    let byte_index = (target_y * bytes_per_row + target_x / 8) as usize;
                    let bit_offset = 7 - (target_x % 8);

                    if byte_index < char_data.len() {
                        char_data[byte_index] |= 1 << bit_offset;
                    }
                }
            }
        }

        // 显示进度
        if index % 500 == 0 {
            progress.update_progress(index, chars.len(), "渲染全角字体");
        }
    }

    // 预览
    preview_string(
        "你好，世界！",
        &result,
        &char_mapping,
        config.font_size,
        config.font_size,
        "全角",
    );

    println!(
        "cargo:warning=  全角字体渲染完成，共渲染字符：{}个",
        char_mapping.len()
    );
    Ok((result, char_mapping))
}

/// 渲染半角字体
fn render_half_width_font(
    config: &BuildConfig,
    chars: &[char],
    progress: &ProgressTracker,
) -> Result<(Vec<u8>, BTreeMap<char, u32>)> {
    let lib = Library::init().map_err(|e| anyhow!("初始化freetype库失败: {}", e))?;
    let face = lib
        .new_face(&config.font_path, 0)
        .map_err(|e| anyhow!("加载字体文件失败 '{}': {}", config.font_path.display(), e))?;

    face.set_pixel_sizes(0, config.font_size)
        .map_err(|e| anyhow!("设置字体大小失败: {}", e))?;

    let mut result = Vec::new();
    let mut char_mapping = BTreeMap::new();

    let target_width = config.font_size / 2;
    let bytes_per_row = (target_width + 7) / 8;
    let char_data_size = (bytes_per_row * config.font_size) as usize;

    for (index, &c) in chars.iter().enumerate() {
        let Some(glyph_index) = face.get_char_index(c as usize) else {
            continue;
        };

        if glyph_index == 0 {
            continue;
        }

        face.load_glyph(glyph_index, freetype::face::LoadFlag::RENDER)
            .map_err(|e| anyhow!("加载字形失败 '{}': {}", c, e))?;

        let glyph = face.glyph();
        let bitmap = glyph.bitmap();

        let bitmap_width = bitmap.width();
        let bitmap_rows = bitmap.rows();
        let bitmap_pitch = bitmap.pitch();

        char_mapping.insert(c, (result.len() / char_data_size) as u32);

        let current_len = result.len();
        result.resize(current_len + char_data_size, 0);

        let char_data = &mut result[current_len..current_len + char_data_size];
        char_data.fill(0);

        let x_offset = if (bitmap_width as u32) < target_width {
            (target_width - bitmap_width as u32) / 2
        } else {
            0
        };

        let y_offset = if (bitmap_rows as u32) < config.font_size {
            (config.font_size - bitmap_rows as u32) / 2
        } else {
            0
        };

        for y in 0..(bitmap_rows as u32) {
            let target_y = y + y_offset;
            if target_y >= config.font_size {
                break;
            }

            for x in 0..(bitmap_width as u32) {
                let target_x = x + x_offset;
                if target_x >= target_width {
                    break;
                }

                let src_x = x as i32;
                let src_y = y as i32;

                let pixel_index = (src_y * bitmap_pitch.abs()) + src_x;
                if pixel_index < 0 || (pixel_index as usize) >= bitmap.buffer().len() {
                    continue;
                }

                let pixel_value = bitmap.buffer()[pixel_index as usize];

                if pixel_value > 128 {
                    let byte_index = (target_y * bytes_per_row + target_x / 8) as usize;
                    let bit_offset = 7 - (target_x % 8);

                    if byte_index < char_data.len() {
                        char_data[byte_index] |= 1 << bit_offset;
                    }
                }
            }
        }

        // 显示进度
        if index % 200 == 0 {
            progress.update_progress(index, chars.len(), "渲染半角字体");
        }
    }

    // 预览
    preview_string(
        "Hello World!",
        &result,
        &char_mapping,
        config.font_size / 2,
        config.font_size,
        "半角",
    );

    println!(
        "cargo:warning=  半角字体渲染完成，共渲染字符：{}个",
        char_mapping.len()
    );
    Ok((result, char_mapping))
}

/// 字符串预览功能
fn preview_string(
    s: &str,
    glyph_data: &[u8],
    char_mapping: &BTreeMap<char, u32>,
    char_width: u32,
    char_height: u32,
    font_type: &str,
) {
    let bytes_per_row = (char_width + 7) / 8;
    let char_data_size = (bytes_per_row * char_height) as usize;

    println!("cargo:warning=  {}字体预览 '{}':", font_type, s);

    for row in 0..char_height {
        let mut line = String::new();
        for c in s.chars() {
            if let Some(&char_index) = char_mapping.get(&c) {
                let start = (char_index as usize) * char_data_size;
                let char_data = &glyph_data[start..start + char_data_size];

                for x in 0..char_width {
                    let byte_index = (row * bytes_per_row + x / 8) as usize;
                    let bit_offset = 7 - (x % 8);

                    let pixel = if byte_index < char_data.len() {
                        (char_data[byte_index] >> bit_offset) & 1
                    } else {
                        0
                    };

                    line.push(if pixel == 1 { '█' } else { ' ' });
                }
            } else {
                line.push_str(&" ".repeat(char_width as usize));
            }
        }
        println!("cargo:warning=  {}", line);
    }
    println!("cargo:warning=");
}

/// 生成字体文件
fn generate_font_files(
    config: &BuildConfig,
    full_width_glyph_data: &[u8],
    full_width_char_mapping: &BTreeMap<char, u32>,
    half_width_glyph_data: &[u8],
    half_width_char_mapping: &BTreeMap<char, u32>,
) -> Result<()> {
    // 生成二进制字体文件
    let full_width_bin_path = config.output_dir.join("full_width_font.bin");
    utils::file_utils::write_file(&full_width_bin_path, full_width_glyph_data)?;

    let half_width_bin_path = config.output_dir.join("half_width_font.bin");
    utils::file_utils::write_file(&half_width_bin_path, half_width_glyph_data)?;

    // 生成字体描述文件
    let fonts_rs_path = config.output_dir.join("fonts.rs");
    let content =
        generate_fonts_rs_content(config, full_width_char_mapping, half_width_char_mapping)?;

    utils::file_utils::write_string_file(&fonts_rs_path, &content)?;

    println!(
        "cargo:warning=  字体文件生成成功: {}",
        fonts_rs_path.display()
    );
    Ok(())
}

/// 生成字体描述文件内容
fn generate_fonts_rs_content(
    config: &BuildConfig,
    full_width_char_mapping: &BTreeMap<char, u32>,
    half_width_char_mapping: &BTreeMap<char, u32>,
) -> Result<String> {
    let mut content = String::new();

    content.push_str("// 自动生成的字体描述文件\n// 不要手动修改此文件\n\n");
    content.push_str("use embedded_graphics::{\n    image::ImageRaw,\n    mono_font::{DecorationDimensions, MonoFont, mapping::GlyphMapping},\n    pixelcolor::BinaryColor,\n    prelude::Size,\n};\n\n");

    content.push_str("struct BinarySearchGlyphMapping {\n    chars: &'static [u16],\n    offsets: &'static [u32],\n}\n\n");

    content.push_str("impl GlyphMapping for BinarySearchGlyphMapping {\n    fn index(&self, c: char) -> usize {\n        let target = c as u16;\n        let mut left = 0;\n        let mut right = self.chars.len();\n        \n        while left < right {\n            let mid = left + (right - left) / 2;\n            if self.chars[mid] < target {\n                left = mid + 1;\n            } else if self.chars[mid] > target {\n                right = mid;\n            } else {\n                return self.offsets[mid] as usize;\n            }\n        }\n        \n        0\n    }\n}\n\n");

    // 生成全角字符映射
    let full_width_offsets: Vec<u32> = full_width_char_mapping.values().cloned().collect();
    content.push_str("const FULL_WIDTH_CHARS: &[u16] = &[\n");
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

    content.push_str("const FULL_WIDTH_OFFSETS: &[u32] = &[\n");
    for &offset in &full_width_offsets {
        content.push_str(&format!("    {},\n", offset));
    }
    content.push_str("];\n\n");

    // 生成半角字符映射
    let half_width_offsets: Vec<u32> = half_width_char_mapping.values().cloned().collect();
    content.push_str("const HALF_WIDTH_CHARS: &[u16] = &[\n");
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

    content.push_str("const HALF_WIDTH_OFFSETS: &[u32] = &[\n");
    for &offset in &half_width_offsets {
        content.push_str(&format!("    {},\n", offset));
    }
    content.push_str("];\n\n");

    content.push_str("static FULL_WIDTH_GLYPH_MAPPING: BinarySearchGlyphMapping = BinarySearchGlyphMapping {\n    chars: FULL_WIDTH_CHARS,\n    offsets: FULL_WIDTH_OFFSETS,\n};\n\n");

    content.push_str("static HALF_WIDTH_GLYPH_MAPPING: BinarySearchGlyphMapping = BinarySearchGlyphMapping {\n    chars: HALF_WIDTH_CHARS,\n    offsets: HALF_WIDTH_OFFSETS,\n};\n\n");

    content.push_str(&format!(
        "pub const FULL_WIDTH_FONT: MonoFont<'static> = MonoFont {{\n    image: ImageRaw::<BinaryColor>::new(\n        include_bytes!(\"full_width_font.bin\"),\n        {}\n    ),\n    glyph_mapping: &FULL_WIDTH_GLYPH_MAPPING,\n    character_size: Size::new({}, {}),\n    character_spacing: 0,\n    baseline: {},\n    underline: DecorationDimensions::new({} + 2, 1),\n    strikethrough: DecorationDimensions::new({} / 2, 1),\n}};\n\n",
        config.font_size, config.font_size, config.font_size, config.font_size, config.font_size, config.font_size
    ));

    content.push_str(&format!(
        "pub const HALF_WIDTH_FONT: MonoFont<'static> = MonoFont {{\n    image: ImageRaw::<BinaryColor>::new(\n        include_bytes!(\"half_width_font.bin\"),\n        {}\n    ),\n    glyph_mapping: &HALF_WIDTH_GLYPH_MAPPING,\n    character_size: Size::new({}, {}),\n    character_spacing: 0,\n    baseline: {},\n    underline: DecorationDimensions::new({} + 2, 1),\n    strikethrough: DecorationDimensions::new({} / 2, 1),\n}};\n",
        config.font_size / 2, config.font_size / 2, config.font_size, config.font_size, config.font_size / 2, config.font_size
    ));

    Ok(content)
}
