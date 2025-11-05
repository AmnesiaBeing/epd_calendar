use anyhow::{Context, Result, anyhow};
use embedded_graphics::mono_font::DecorationDimensions;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::Size;
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use unicode_normalization::UnicodeNormalization;

// 字体大小，中文一定是SIZE*SIZE，英文和特殊符号是SIZE/2（半宽）*SIZE
const FONT_SIZE: u32 = 16;

// 资源目录
const OUTPUT_DIR: &str = "src/generated";
const SENTENCES_DIR: &str = "../sentences-bundle/sentences";
const CATEGORIES_PATH: &str = "../sentences-bundle/categories.json";
const FONT_PATH: &str = "assets/SimSun.ttf";

// sentences-bundle 数据 JSON解析结构
#[derive(Debug, Deserialize)]
struct Hitokoto {
    hitokoto: String,
    from: String,
    from_who: Option<String>,
}

// sentences-bundle 分类 JSON解析结构
#[derive(Debug, Deserialize)]
struct HitokotoCategory {
    id: u32,      // 要把Hitokoto中的type通过key转换为id，方便存储
    name: String, // 分类名称
    key: String,  // 分类缩写，abcde……
}

fn main() -> Result<()> {
    // 1. 解析分类
    let categories = parse_categories()?;

    // 2. 解析一言数据
    let hitokotos = parse_all_json_files(&categories)?;

    // 3. 收集并过滤字符（控制字符、非BMP字符）
    let valid_chars: Vec<Hitokoto> = hitokotos.into_iter().flat_map(|(_, v)| v).collect();
    let (all_chars, char_to_idx) = collect_and_validate_chars(&valid_chars)?;
    eprintln!("共收集有效字符：{}个", all_chars.len());

    // 4. 渲染字体点阵（使用ttf-parser，替代font-rs避免溢出）
    let glyph_data = render_font_bitmaps(&all_chars)?;

    // 5. 生成字符串引用（映射一言数据到字符索引）
    let (str_refs, global_char_indices) = generate_string_refs(&valid_chars, &char_to_idx)?;

    // 6. 生成适配embedded-graphics的代码
    // generate_embedded_code(
    //     &hitokotos,
    //     &categories,
    //     &all_chars,
    //     &glyph_data,
    //     baseline,
    //     &str_refs,
    //     &global_char_indices,
    //     FONT_SIZE,
    // )?;

    // 触发重编译的条件
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed={}", SENTENCES_DIR);
    println!("cargo:rerun-if-changed={}", CATEGORIES_PATH);
    println!("cargo:rerun-if-changed={}", FONT_PATH);

    Ok(())
}

// 解析分类文件
fn parse_categories() -> Result<Vec<HitokotoCategory>> {
    let categories: Vec<HitokotoCategory> = serde_json::from_str(
        &fs::read_to_string(CATEGORIES_PATH)
            .with_context(|| format!("读取分类文件失败: {}", CATEGORIES_PATH))?,
    )
    .context("解析categories.json失败")?;

    Ok(categories)
}

// 解析一言JSON文件
// 返回值：type + 不同type格言数组
fn parse_all_json_files(categories: &[HitokotoCategory]) -> Result<Vec<(u32, Vec<Hitokoto>)>> {
    let mut vec_hitokotos = Vec::new();

    for entry in categories {
        let mut sentences_path = PathBuf::from(SENTENCES_DIR);
        sentences_path.push(entry.key.clone());
        sentences_path.set_extension("json");

        let content = fs::read_to_string(sentences_path.as_path())
            .with_context(|| format!("读取JSON文件失败: {}", sentences_path.clone().display()))?;

        let hitokotos: Vec<Hitokoto> = serde_json::from_str(&content)
            .with_context(|| format!("解析JSON失败: {}", sentences_path.display()))?;

        vec_hitokotos.push((entry.id, hitokotos));
    }

    Ok(vec_hitokotos)
}

// 收集并过滤字符（控制字符、非BMP字符）
fn collect_and_validate_chars(hitokotos: &[Hitokoto]) -> Result<(Vec<char>, HashMap<char, usize>)> {
    let mut char_set = BTreeSet::new();

    // 强制加入常见英文、特殊符号、空格
    for h in r#"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ1234567890~!@#$%^&*()-_=+[{}]\|;:'",<.>/?/* "#.chars() {
        char_set.insert(h);
    }

    for h in hitokotos {
        for c in h.from.nfc() {
            char_set.insert(c);
        }
        for c in h.from_who.clone().unwrap_or("佚名".to_string()).nfc() {
            char_set.insert(c);
        }
        for c in h.hitokoto.nfc() {
            char_set.insert(c);
        }
    }

    // BTree转HashMap
    let mut all_chars = Vec::new();
    let mut char_to_idx = HashMap::new();
    for (idx, c) in char_set.into_iter().enumerate() {
        all_chars.push(c);
        char_to_idx.insert(c, idx);
    }

    Ok((all_chars, char_to_idx))
}

// 判断是否为ASCII、CJK字符，并且要在0xFFFF以下（系统只使用UTF-8和Unicode16，不处理超过的字符）
fn is_valid_char(c: char) -> bool {
    let code = c as u32;
    let ret = code <= 0xFFFF
        && ((code >= 0x0032 && code <= 0x007E) || /* ASCII */
        (code >= 0x4E00 && code <= 0x9FFF) || /* 基本汉字 */
        (code >= 0x3400 && code <= 0x4DBF) || /* 扩展A */
        (code >= 0x2F00 && code <= 0x2FD5) || /* 康熙部首 */
        (code >= 0x3001 && code <= 0x303D)/* 中日韩标点符号 */);

    if !ret {
        eprintln!("警告: 字符 '{}' (U+{:04X}) 不为所需字符，已过滤", c, code);
    }

    ret
}

// ASCII英文字母、数字、特殊符号占一半宽度，中文常见的结束符号也占据一半宽度
fn if_half_width_char(c: char) -> bool {
    c.is_ascii() || r#"，。！？、"#.to_string().find(c).is_some()
}

// 渲染字体点阵
fn render_font_bitmaps(chars: &[char]) -> Result<Vec<u8>> {
    use fontdue::Font;

    // 1. 加载TTF字体
    let font_data = fs::read(FONT_PATH).context("读取字体文件失败")?;
    let font = Font::from_bytes(font_data.as_slice(), fontdue::FontSettings::default())
        .map_err(|e| anyhow!("加载字体失败: {}", e))?;

    // 2. 计算单个字符的点阵尺寸，为了排版简单，ASCII范围内的字符宽度为其他范围的一半
    let width = FONT_SIZE;
    let half_width = FONT_SIZE / 2;
    let height = FONT_SIZE;
    let bytes_per_row = (width + 7) / 8; // 向上取整（如16宽 → 2字节/行）
    let bytes_per_glyph = bytes_per_row * height;

    let mut glyph_data = Vec::with_capacity(chars.len() * bytes_per_glyph as usize);

    for &c in chars {
        // 4. 获取字符的Glyph索引和度量
        let glyph_index = font.lookup_glyph_index(c);

        // 5. 光栅化（生成灰度点阵）
        let (metrics, bitmap) = font.rasterize(c, FONT_SIZE as f32);

        // 6. 转换为二值化点阵并居中对齐
        let mut glyph_bits = vec![0u8; bytes_per_glyph as usize];

        // 计算居中偏移（水平和垂直居中）
        let offset_x = (width as i32 - metrics.width as i32) / 2;
        let offset_y = (height as i32 - metrics.height as i32) / 2;

        // 遍历光栅化后的像素
        for (y, row) in bitmap.chunks(metrics.width).enumerate() {
            for (x, &gray) in row.iter().enumerate() {
                // 计算目标位置（考虑居中偏移）
                let target_x = x as i32 + offset_x;
                let target_y = y as i32 + offset_y;

                // 检查是否在目标范围内
                if target_x >= 0
                    && target_x < width as i32
                    && target_y >= 0
                    && target_y < height as i32
                {
                    let tx = target_x as u32;
                    let ty = target_y as u32;

                    // 二值化：灰度>127视为有效像素（1）
                    if gray > 127 {
                        let byte_idx = (ty * bytes_per_row + tx / 8) as usize;
                        let bit_pos = 7 - (tx % 8); // 高位在前（BigEndian）
                        glyph_bits[byte_idx] |= 1 << bit_pos;
                    }
                }
            }
        }

        glyph_data.extend(glyph_bits);
    }

    Ok(glyph_data)
}

// 生成字符串引用（映射到字符索引）
fn generate_string_refs(
    hitokotos: &[Hitokoto],
    char_to_idx: &HashMap<char, usize>,
) -> Result<(Vec<(usize, usize, usize, usize, usize, usize)>, Vec<usize>)> {
    let mut global_char_indices = Vec::new();
    let mut str_refs = Vec::with_capacity(hitokotos.len());

    for h in hitokotos {
        // 生成from字段的索引序列
        let from_indices: Vec<usize> = h
            .from
            .nfc()
            .map(|c| {
                *char_to_idx
                    .get(&c)
                    .with_context(|| format!("from字段存在未收集字符: '{}'", c))
                    .unwrap()
            })
            .collect();

        // 生成from_who字段的索引序列
        let from_who_indices: Vec<usize> = h
            .from_who
            .clone()
            .unwrap_or("佚名".to_string())
            .nfc()
            .map(|c| {
                *char_to_idx
                    .get(&c)
                    .with_context(|| format!("from_who字段存在未收集字符: '{}'", c))
                    .unwrap()
            })
            .collect();

        // 生成hitokoto字段的索引序列
        let hitokoto_indices: Vec<usize> = h
            .hitokoto
            .nfc()
            .map(|c| {
                *char_to_idx
                    .get(&c)
                    .with_context(|| format!("hitokoto字段存在未收集字符: '{}'", c))
                    .unwrap()
            })
            .collect();

        // 记录起始索引和长度
        let from_start = global_char_indices.len();
        let from_len = from_indices.len();
        global_char_indices.extend(from_indices);

        let from_who_start = global_char_indices.len();
        let from_who_len = from_who_indices.len();
        global_char_indices.extend(from_who_indices);

        let content_start = global_char_indices.len();
        let content_len = hitokoto_indices.len();
        global_char_indices.extend(hitokoto_indices);

        str_refs.push((
            from_start,
            from_len,
            from_who_start,
            from_who_len,
            content_start,
            content_len,
        ));
    }

    Ok((str_refs, global_char_indices))
}

// // 生成适配embedded-graphics的代码
// fn generate_embedded_code(
//     hitokotos: &[ProcessedHitokoto],
//     categories: &[CategoryWithEnumName],
//     all_chars: &[char],
//     glyph_data: &[u8],
//     baseline: u32,
//     str_refs: &[(usize, usize, usize, usize, usize, usize)],
//     global_char_indices: &[usize],
//     font_size: u32,
// ) -> Result<()> {
//     let output_dir = Path::new(OUTPUT_DIR);
//     fs::create_dir_all(output_dir).context("创建输出目录失败")?;
//     let output_path = output_dir.join("hitokoto_data.rs");

//     let mut code = String::new();

//     // 生成头部注释
//     code.push_str(
//         "//! 自动生成的一言数据和嵌入式字体（适配embedded-graphics）
// //! 由build.rs生成，请勿手动修改
// use embedded_graphics::mono_font::{GlyphMapping, MonoFont};
// use embedded_graphics::image::ImageRaw;
// use embedded_graphics::pixelcolor::BinaryColor;
// use embedded_graphics::prelude::{Point, Size};
// use embedded_graphics::mono_font::DecorationDimensions;
// use core::cmp::Ordering;

// ",
//     );

//     // 1. 生成HitokotoType枚举
//     code.push_str("/// 一言类型枚举\n");
//     code.push_str("#[derive(Debug, Clone, Copy, PartialEq, Eq)]\n");
//     code.push_str("pub enum HitokotoType {\n");
//     for c in categories {
//         code.push_str(&format!("    /// {}\n    {},\n", c.name, c.enum_name));
//     }
//     code.push_str("}\n\n");

//     // 2. 生成字符串引用结构体
//     code.push_str("/// 字符串引用：表示字符串在全局字符索引数组中的范围\n");
//     code.push_str("#[derive(Debug, Clone, Copy, PartialEq, Eq)]\n");
//     code.push_str("pub struct StrRef {\n");
//     code.push_str("    pub start: usize,\n");
//     code.push_str("    pub len: usize,\n");
//     code.push_str("}\n\n");

//     // 3. 生成一言数据结构体
//     code.push_str("/// 一言数据结构体\n");
//     code.push_str("#[derive(Debug, Clone, Copy, PartialEq, Eq)]\n");
//     code.push_str("pub struct Hitokoto {\n");
//     code.push_str("    pub type_: HitokotoType,\n");
//     code.push_str("    pub from: StrRef,\n");
//     code.push_str("    pub from_who: StrRef,\n");
//     code.push_str("    pub content: StrRef,\n");
//     code.push_str("}\n\n");

//     // 4. 生成字体数据（MonoFont）
//     let char_size = Size {
//         width: font_size,
//         height: font_size,
//     };
//     let bytes_per_row = (char_size.width + 7) / 8;

//     code.push_str("/// 所有字符列表（按Unicode排序，用于GlyphMapping）\n");
//     code.push_str("pub const ALL_CHARS: &[char] = &[\n");
//     for &c in all_chars {
//         code.push_str(&format!(
//             "    '{}', // U+{:04X}\n",
//             c.escape_default(),
//             c as u32
//         ));
//     }
//     code.push_str("];\n\n");

//     code.push_str("/// 字体点阵数据（适配ImageRaw<BinaryColor>）\n");
//     code.push_str("pub const FONT_DATA: &[u8] = &[\n");
//     for (i, &byte) in glyph_data.iter().enumerate() {
//         if i % 16 == 0 {
//             code.push_str("    ");
//         }
//         code.push_str(&format!("0x{:02X}, ", byte));
//         if (i + 1) % 16 == 0 {
//             code.push_str("\n");
//         }
//     }
//     code.push_str("];\n\n");

//     code.push_str("/// embedded-graphics兼容的MonoFont字体\n");
//     code.push_str("pub const EMBEDDED_FONT: MonoFont = MonoFont {\n");
//     code.push_str(&format!(
//         "    image: ImageRaw::<BinaryColor>::new(FONT_DATA, {}),\n",
//         char_size.width
//     ));
//     code.push_str(&format!(
//         "    character_size: Size {{ width: {}, height: {} }},\n",
//         char_size.width, char_size.height
//     ));
//     code.push_str("    character_spacing: 0,\n"); // 无额外间距
//     code.push_str(&format!("    baseline: {},\n", baseline));
//     code.push_str("    strikethrough: DecorationDimensions { offset: 0, height: 0 },\n");
//     code.push_str("    underline: DecorationDimensions { offset: 0, height: 0 },\n");
//     code.push_str("    glyph_mapping: &HitokotoGlyphMapping,\n");
//     code.push_str("};\n\n");

//     // 5. 实现自定义GlyphMapping（二分查找）
//     code.push_str("/// 基于二分查找的GlyphMapping实现（适配3500字符规模）\n");
//     code.push_str("#[derive(Debug)]\n");
//     code.push_str("pub struct HitokotoGlyphMapping;\n\n");
//     code.push_str("impl GlyphMapping for HitokotoGlyphMapping {\n");
//     code.push_str("    fn index(&self, c: char) -> usize {\n");
//     code.push_str("        // 二分查找字符在ALL_CHARS中的索引\n");
//     code.push_str("        let mut low = 0;\n");
//     code.push_str("        let mut high = ALL_CHARS.len();\n");
//     code.push_str("        while low < high {\n");
//     code.push_str("            let mid = (low + high) / 2;\n");
//     code.push_str("            match ALL_CHARS[mid].cmp(&c) {\n");
//     code.push_str("                Ordering::Less => low = mid + 1,\n");
//     code.push_str("                Ordering::Greater => high = mid,\n");
//     code.push_str("                Ordering::Equal => return mid,\n");
//     code.push_str("            }\n");
//     code.push_str("        }\n");
//     code.push_str("        // 未找到时返回空格的索引（确保空格在ALL_CHARS中）\n");
//     code.push_str("        ALL_CHARS.iter().position(|&ch| ch == ' ').unwrap_or(0)\n");
//     code.push_str("    }\n");
//     code.push_str("}\n\n");

//     // 6. 生成全局字符索引数组
//     code.push_str("/// 所有字符串的字符索引序列（映射到ALL_CHARS）\n");
//     code.push_str("pub const GLOBAL_CHAR_INDICES: &[usize] = &[\n");
//     for (i, &idx) in global_char_indices.iter().enumerate() {
//         if i % 16 == 0 {
//             code.push_str("    ");
//         }
//         code.push_str(&format!("{}, ", idx));
//         if (i + 1) % 16 == 0 {
//             code.push_str("\n");
//         }
//     }
//     code.push_str("];\n\n");

//     // 7. 生成一言数据数组
//     code.push_str("/// 所有一言数据\n");
//     code.push_str("pub const ALL_HITOKOTOS: &[Hitokoto] = &[\n");
//     for (i, &(from_start, from_len, from_who_start, from_who_len, content_start, content_len)) in
//         str_refs.iter().enumerate()
//     {
//         let h = &hitokotos[i];
//         code.push_str(&format!(
//             "    Hitokoto {{\n        type_: HitokotoType::{},\n",
//             h.type_enum_name
//         ));
//         code.push_str(&format!(
//             "        from: StrRef {{ start: {}, len: {} }},\n",
//             from_start, from_len
//         ));
//         code.push_str(&format!(
//             "        from_who: StrRef {{ start: {}, len: {} }},\n",
//             from_who_start, from_who_len
//         ));
//         code.push_str(&format!(
//             "        content: StrRef {{ start: {}, len: {} }},\n    }},\n",
//             content_start, content_len
//         ));
//     }
//     code.push_str("];\n\n");

//     // 8. 生成辅助函数
//     code.push_str("/// 通过StrRef获取字符索引序列\n");
//     code.push_str("pub fn str_ref_to_indices(s: StrRef) -> &'static [usize] {\n");
//     code.push_str("    &GLOBAL_CHAR_INDICES[s.start..s.start + s.len]\n");
//     code.push_str("}\n");

//     // 写入文件
//     fs::write(&output_path, code)
//         .with_context(|| format!("写入生成代码失败: {}", output_path.display()))?;

//     Ok(())
// }
