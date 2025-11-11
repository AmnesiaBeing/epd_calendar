use anyhow::{Context, Result, anyhow};
use freetype::Library;
use freetype::bitmap::PixelMode;
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use unicode_normalization::UnicodeNormalization;

const FONT_SIZE: u32 = 24;
const OUTPUT_DIR: &str = "src/app";
const SENTENCES_DIR: &str = "../sentences-bundle/sentences";
const CATEGORIES_PATH: &str = "../sentences-bundle/categories.json";
const FONT_PATH: &str = "assets/NotoSansMonoCJKsc-Regular.otf";

#[derive(Debug, Deserialize, Clone)]
struct Hitokoto {
    hitokoto: String,
    from: String,
    from_who: Option<String>,
}

#[derive(Debug, Deserialize)]
struct HitokotoCategory {
    id: u32,
    #[allow(dead_code)]
    name: String,
    key: String,
}

fn main() -> Result<()> {
    println!("cargo:warning=开始构建字体数据...");

    let categories = parse_categories()?;
    let hitokotos = parse_all_json_files(&categories)?;
    let all_chars: Vec<Hitokoto> = hitokotos.clone().into_iter().flat_map(|(_, v)| v).collect();
    let all_chars = collect_and_validate_chars(&all_chars)?;

    println!("cargo:warning=共收集有效字符：{}个", all_chars.len());

    let (full_width_chars, half_width_chars) = separate_full_half_width_chars(&all_chars);
    let half_width_chars = add_ascii_chars(half_width_chars);
    let full_width_chars = ensure_yiming_chars(full_width_chars);

    println!(
        "cargo:warning=全角字符：{}个，半角字符：{}个",
        full_width_chars.len(),
        half_width_chars.len()
    );

    let (full_width_glyph_data, full_width_char_mapping) =
        render_full_width_font(&full_width_chars)?;
    let (half_width_glyph_data, half_width_char_mapping) =
        render_half_width_font(&half_width_chars)?;

    generate_hitokoto_data(&hitokotos)?;
    generate_font_files(
        &full_width_glyph_data,
        &full_width_char_mapping,
        &half_width_glyph_data,
        &half_width_char_mapping,
    )?;

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed={}", SENTENCES_DIR);
    println!("cargo:rerun-if-changed={}", CATEGORIES_PATH);
    println!("cargo:rerun-if-changed={}", FONT_PATH);

    println!("cargo:warning=字体数据构建完成！");
    Ok(())
}

fn parse_categories() -> Result<Vec<HitokotoCategory>> {
    let content = fs::read_to_string(CATEGORIES_PATH)
        .with_context(|| format!("读取分类文件失败: {}", CATEGORIES_PATH))?;
    let categories: Vec<HitokotoCategory> =
        serde_json::from_str(&content).context("解析categories.json失败")?;
    Ok(categories)
}

fn parse_all_json_files(categories: &[HitokotoCategory]) -> Result<Vec<(u32, Vec<Hitokoto>)>> {
    let mut vec_hitokotos = Vec::new();

    for entry in categories {
        let mut sentences_path = PathBuf::from(SENTENCES_DIR);
        sentences_path.push(entry.key.clone());
        sentences_path.set_extension("json");

        let content = fs::read_to_string(&sentences_path)
            .with_context(|| format!("读取JSON文件失败: {}", sentences_path.display()))?;

        let hitokotos: Vec<Hitokoto> = serde_json::from_str(&content)
            .with_context(|| format!("解析JSON失败: {}", sentences_path.display()))?;

        vec_hitokotos.push((entry.id, hitokotos));
    }

    Ok(vec_hitokotos)
}

fn collect_and_validate_chars(hitokotos: &[Hitokoto]) -> Result<Vec<char>> {
    let mut char_set = BTreeSet::new();

    for h in hitokotos {
        for c in h.from.nfc() {
            char_set.insert(c);
        }
        for c in h
            .from_who
            .clone()
            .unwrap_or_else(|| "佚名".to_string())
            .nfc()
        {
            char_set.insert(c);
        }
        for c in h.hitokoto.nfc() {
            char_set.insert(c);
        }
    }

    let filtered_chars: Vec<char> = char_set
        .into_iter()
        .filter(|&c| !is_invisible_char(c))
        .collect();

    Ok(filtered_chars)
}

fn is_invisible_char(c: char) -> bool {
    match c as u32 {
        0x20 | 0x09 | 0x0A | 0x0D => true,
        0xA0 | 0xAD | 0x3000 => true,
        n if n <= 0x1F || (n >= 0x7F && n <= 0x9F) => true,
        _ => false,
    }
}

fn separate_full_half_width_chars(chars: &[char]) -> (Vec<char>, Vec<char>) {
    let mut full_width_chars = Vec::new();
    let mut half_width_chars = Vec::new();

    for &c in chars {
        if if_half_width_char(c) {
            half_width_chars.push(c);
        } else {
            full_width_chars.push(c);
        }
    }

    (full_width_chars, half_width_chars)
}

fn ensure_yiming_chars(full_width_chars: Vec<char>) -> Vec<char> {
    let mut char_set: BTreeSet<char> = full_width_chars.into_iter().collect();
    char_set.insert('佚');
    char_set.insert('名');
    char_set.into_iter().collect()
}

fn if_half_width_char(c: char) -> bool {
    // 只保留英文字母、数字、标点符号为半角
    c.is_ascii() && !c.is_ascii_control()
}

fn add_ascii_chars(existing_chars: Vec<char>) -> Vec<char> {
    let mut all_chars: BTreeSet<char> = existing_chars.into_iter().collect();

    for code in 0x21..=0x7E {
        if let Some(c) = std::char::from_u32(code) {
            all_chars.insert(c);
        }
    }

    all_chars.into_iter().collect()
}

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

    println!("cargo:warning={}字体预览 '{}':", font_type, s);

    // 按行预览
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
                // 字符不在映射中，显示空格
                line.push_str(&" ".repeat(char_width as usize));
            }
        }
        println!("cargo:warning={}", line);
    }
    println!("cargo:warning=");
}

fn render_full_width_font(chars: &[char]) -> Result<(Vec<u8>, BTreeMap<char, u32>)> {
    let lib = Library::init().map_err(|e| anyhow!("初始化freetype库失败: {}", e))?;
    let face = lib
        .new_face(FONT_PATH, 0)
        .map_err(|e| anyhow!("加载字体文件失败 '{}': {}", FONT_PATH, e))?;

    face.set_pixel_sizes(0, FONT_SIZE)
        .map_err(|e| anyhow!("设置字体大小失败: {}", e))?;

    let mut result = Vec::new();
    let mut char_mapping = BTreeMap::new();

    let bytes_per_row = (FONT_SIZE + 7) / 8;
    let char_data_size = (bytes_per_row * FONT_SIZE) as usize;

    for (index, &c) in chars.iter().enumerate() {
        let Some(glyph_index) = face.get_char_index(c as usize) else {
            println!(
                "cargo:warning=字符 '{}' (U+{:04X}) 没有对应的字形索引，已过滤",
                c, c as u32
            );
            continue;
        };

        if glyph_index == 0 {
            println!(
                "cargo:warning=字符 '{}' (U+{:04X}) 没有对应的字形索引，已过滤",
                c, c as u32
            );
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

        let x_offset = if (bitmap_width as u32) < FONT_SIZE {
            (FONT_SIZE - bitmap_width as u32) / 2
        } else {
            0
        };

        let y_offset = if (bitmap_rows as u32) < FONT_SIZE {
            (FONT_SIZE - bitmap_rows as u32) / 2
        } else {
            0
        };

        for y in 0..(bitmap_rows as u32) {
            let target_y = y + y_offset;
            if target_y >= FONT_SIZE {
                break;
            }

            for x in 0..(bitmap_width as u32) {
                let target_x = x + x_offset;
                if target_x >= FONT_SIZE {
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
        if index % 100 == 0 {
            println!("cargo:warning=全角字体渲染进度: {}/{}", index, chars.len());
        }
    }

    // 预览指定字符串
    preview_string(
        "你好，世界！",
        &result,
        &char_mapping,
        FONT_SIZE,
        FONT_SIZE,
        "全角",
    );

    println!(
        "cargo:warning=全角字体渲染完成，共渲染字符：{}个",
        char_mapping.len()
    );
    Ok((result, char_mapping))
}

fn render_half_width_font(chars: &[char]) -> Result<(Vec<u8>, BTreeMap<char, u32>)> {
    let lib = Library::init().map_err(|e| anyhow!("初始化freetype库失败: {}", e))?;
    let face = lib
        .new_face(FONT_PATH, 0)
        .map_err(|e| anyhow!("加载字体文件失败 '{}': {}", FONT_PATH, e))?;

    face.set_pixel_sizes(0, FONT_SIZE)
        .map_err(|e| anyhow!("设置字体大小失败: {}", e))?;

    let mut result = Vec::new();
    let mut char_mapping = BTreeMap::new();

    let target_width = FONT_SIZE / 2;
    let bytes_per_row = (target_width + 7) / 8;
    let char_data_size = (bytes_per_row * FONT_SIZE) as usize;

    for (index, &c) in chars.iter().enumerate() {
        let Some(glyph_index) = face.get_char_index(c as usize) else {
            println!(
                "cargo:warning=字符 '{}' (U+{:04X}) 没有对应的字形索引，已过滤",
                c, c as u32
            );
            continue;
        };

        if glyph_index == 0 {
            println!(
                "cargo:warning=字符 '{}' (U+{:04X}) 没有对应的字形索引，已过滤",
                c, c as u32
            );
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

        let y_offset = if (bitmap_rows as u32) < FONT_SIZE {
            (FONT_SIZE - bitmap_rows as u32) / 2
        } else {
            0
        };

        for y in 0..(bitmap_rows as u32) {
            let target_y = y + y_offset;
            if target_y >= FONT_SIZE {
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
        if index % 100 == 0 {
            println!("cargo:warning=半角字体渲染进度: {}/{}", index, chars.len());
        }
    }

    // 预览指定字符串
    preview_string(
        "Hello World!",
        &result,
        &char_mapping,
        FONT_SIZE / 2,
        FONT_SIZE,
        "半角",
    );

    println!(
        "cargo:warning=半角字体渲染完成，共渲染字符：{}个",
        char_mapping.len()
    );
    Ok((result, char_mapping))
}

fn generate_hitokoto_data(hitokotos: &[(u32, Vec<Hitokoto>)]) -> Result<()> {
    let output_path = Path::new(OUTPUT_DIR).join("hitokoto_data.rs");

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
    content.push_str("pub const FROM_STRINGS: &[&str] = &[\n");
    for from_str in &from_vec {
        content.push_str(&format!("    \"{}\",\n", escape_string(from_str)));
    }
    content.push_str("];\n\n");

    content.push_str("pub const FROM_WHO_STRINGS: &[&str] = &[\n");
    for from_who_str in &from_who_vec {
        content.push_str(&format!("    \"{}\",\n", escape_string(from_who_str)));
    }
    content.push_str("];\n\n");

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
            escape_string(&hitokoto.hitokoto)
        ));
        content.push_str(&format!("        from: {},\n", from_index));
        content.push_str(&format!("        from_who: {},\n", from_who_index));
        content.push_str(&format!("        category: {},\n", category_id));
        content.push_str("    },\n");
    }

    content.push_str("];\n");

    fs::write(&output_path, content)
        .with_context(|| format!("写入格言数据文件失败: {}", output_path.display()))?;

    println!(
        "cargo:warning=格言数据文件生成成功: {}",
        output_path.display()
    );
    println!(
        "cargo:warning=来源字符串数量: {}, 作者字符串数量: {}, 格言总数: {}",
        from_vec.len(),
        from_who_vec.len(),
        all_hitokotos.len()
    );

    Ok(())
}

fn generate_font_files(
    full_width_glyph_data: &[u8],
    full_width_char_mapping: &BTreeMap<char, u32>,
    half_width_glyph_data: &[u8],
    half_width_char_mapping: &BTreeMap<char, u32>,
) -> Result<()> {
    let full_width_bin_path = Path::new(OUTPUT_DIR).join("full_width_font.bin");
    fs::write(&full_width_bin_path, full_width_glyph_data).with_context(|| {
        format!(
            "写入全角字体二进制文件失败: {}",
            full_width_bin_path.display()
        )
    })?;

    let half_width_bin_path = Path::new(OUTPUT_DIR).join("half_width_font.bin");
    fs::write(&half_width_bin_path, half_width_glyph_data).with_context(|| {
        format!(
            "写入半角字体二进制文件失败: {}",
            half_width_bin_path.display()
        )
    })?;

    let fonts_rs_path = Path::new(OUTPUT_DIR).join("hitokoto_fonts.rs");

    let mut content = String::new();

    content.push_str("// 自动生成的字体描述文件\n// 不要手动修改此文件\n\n");
    content.push_str("use embedded_graphics::{\n    image::ImageRaw,\n    mono_font::{DecorationDimensions, MonoFont, mapping::GlyphMapping},\n    pixelcolor::BinaryColor,\n    prelude::Size,\n};\n\n");

    content.push_str("struct BinarySearchGlyphMapping {\n    chars: &'static [u16],\n    offsets: &'static [u32],\n}\n\n");

    content.push_str("impl GlyphMapping for BinarySearchGlyphMapping {\n    fn index(&self, c: char) -> usize {\n        let target = c as u16;\n        let mut left = 0;\n        let mut right = self.chars.len();\n        \n        while left < right {\n            let mid = left + (right - left) / 2;\n            if self.chars[mid] < target {\n                left = mid + 1;\n            } else if self.chars[mid] > target {\n                right = mid;\n            } else {\n                return self.offsets[mid] as usize;\n            }\n        }\n        \n        0\n    }\n}\n\n");

    // 生成带注释的全角字符映射
    let full_width_offsets: Vec<u32> = full_width_char_mapping.values().cloned().collect();

    content.push_str("const FULL_WIDTH_CHARS: &[u16] = &[\n");
    for (&c, _) in full_width_char_mapping.iter().take(50) {
        // 只显示前50个字符的注释，避免文件过大
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
    if full_width_char_mapping.len() > 50 {
        content.push_str(&format!(
            "    // ... 还有 {} 个字符\n",
            full_width_char_mapping.len() - 50
        ));
    }
    content.push_str("];\n\n");

    content.push_str("const FULL_WIDTH_OFFSETS: &[u32] = &[\n");
    for &offset in &full_width_offsets {
        content.push_str(&format!("    {},\n", offset));
    }
    content.push_str("];\n\n");

    // 生成带注释的半角字符映射
    let half_width_offsets: Vec<u32> = half_width_char_mapping.values().cloned().collect();

    content.push_str("const HALF_WIDTH_CHARS: &[u16] = &[\n");
    for (&c, _) in half_width_char_mapping.iter().take(50) {
        // 只显示前50个字符的注释
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
    if half_width_char_mapping.len() > 50 {
        content.push_str(&format!(
            "    // ... 还有 {} 个字符\n",
            half_width_char_mapping.len() - 50
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
        FONT_SIZE, FONT_SIZE, FONT_SIZE, FONT_SIZE, FONT_SIZE, FONT_SIZE
    ));

    content.push_str(&format!(
        "pub const HALF_WIDTH_FONT: MonoFont<'static> = MonoFont {{\n    image: ImageRaw::<BinaryColor>::new(\n        include_bytes!(\"half_width_font.bin\"),\n        {}\n    ),\n    glyph_mapping: &HALF_WIDTH_GLYPH_MAPPING,\n    character_size: Size::new({}, {}),\n    character_spacing: 0,\n    baseline: {},\n    underline: DecorationDimensions::new({} + 2, 1),\n    strikethrough: DecorationDimensions::new({} / 2, 1),\n}};\n",
        FONT_SIZE / 2, FONT_SIZE / 2, FONT_SIZE, FONT_SIZE, FONT_SIZE / 2, FONT_SIZE
    ));

    fs::write(&fonts_rs_path, content)
        .with_context(|| format!("写入字体描述文件失败: {}", fonts_rs_path.display()))?;

    println!(
        "cargo:warning=字体描述文件生成成功: {}",
        fonts_rs_path.display()
    );
    Ok(())
}

fn escape_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('\"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}
