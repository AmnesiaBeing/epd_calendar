use anyhow::{Context, Result, anyhow};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use unicode_normalization::UnicodeNormalization;
// 添加freetype-rs的导入
use freetype::Library;

// 字体大小，中文一定是SIZE*SIZE，英文和特殊符号是SIZE/2（半宽）*SIZE
const FONT_SIZE: u32 = 16;

// 资源目录
const OUTPUT_DIR: &str = "src/generated";
const SENTENCES_DIR: &str = "../sentences-bundle/sentences";
const CATEGORIES_PATH: &str = "../sentences-bundle/categories.json";
const FONT_PATH: &str = "assets/SimSun.ttf";

// sentences-bundle 数据 JSON解析结构
#[derive(Debug, Deserialize, Clone)]
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
    let valid_chars: Vec<Hitokoto> = hitokotos.clone().into_iter().flat_map(|(_, v)| v).collect();
    let (all_chars, char_to_idx) = collect_and_validate_chars(&valid_chars)?;
    eprintln!("共收集有效字符：{}个", all_chars.len());

    // 4. 渲染字体点阵（使用ttf-parser，替代font-rs避免溢出）
    let glyph_data = render_font_bitmaps(&all_chars)?;

    // 5. 生成字符串引用（映射一言数据到字符索引）
    let (str_refs, global_char_indices) = generate_string_refs(&valid_chars, &char_to_idx)?;

    // 6. 生成适配embedded-graphics的代码
    generate_embedded_code(
        &hitokotos,
        &categories,
        &all_chars,
        &glyph_data,
        &str_refs,
        &global_char_indices,
        FONT_SIZE,
    )?;

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
    // 初始化freetype库
    let lib = Library::init().map_err(|e| anyhow!("初始化freetype库失败: {}", e))?;

    // 加载字体文件
    let face = lib
        .new_face(FONT_PATH, 0)
        .map_err(|e| anyhow!("加载字体文件失败 '{}': {}", FONT_PATH, e))?;

    // 设置字体大小
    face.set_pixel_sizes(0, FONT_SIZE)
        .map_err(|e| anyhow!("设置字体大小失败: {}", e))?;

    let mut result = Vec::new();

    for &c in chars {
        eprintln!("正在渲染字符: '{}', 字符码: U+{:04X}", c, c as u32);
        // 加载字符的字形
        let glyph_index = face.get_char_index((c as u32).try_into().unwrap());
        if glyph_index.is_none() {
            eprintln!(
                "警告: 字符 '{}' (U+{:04X}) 没有对应的字形索引，已过滤",
                c, c as u32
            );
            continue;
        }

        face.load_glyph(glyph_index.unwrap(), freetype::face::LoadFlag::RENDER)
            .map_err(|e| anyhow!("加载字形失败 '{}': {}", c, e))?;

        // 获取渲染后的位图
        let bitmap = face.glyph().bitmap();

        // 判断是否为半宽字符
        let is_half_width = if_half_width_char(c);
        let target_width = if is_half_width {
            FONT_SIZE / 2
        } else {
            FONT_SIZE
        };

        // 计算每个字符需要的字节数（向上取整到8的倍数）
        let bytes_per_row = (target_width + 7) / 8;
        let char_data_size = bytes_per_row * FONT_SIZE;

        // 确保有足够的空间存储字符数据
        let current_len = result.len();
        result.resize(current_len + char_data_size as usize, 0);

        // 获取当前字符的数据切片
        let char_data = &mut result[current_len..current_len + char_data_size as usize];

        // 计算缩放比例
        let scale_x = bitmap.width() as f32 / target_width as f32;
        let scale_y = bitmap.rows() as f32 / FONT_SIZE as f32;

        // 将位图数据转换为指定大小的黑白数据
        for y in 0..FONT_SIZE {
            for x in 0..target_width {
                // 计算在原始位图中的位置
                let src_x = (x as f32 * scale_x) as u32;
                let src_y = (y as f32 * scale_y) as u32;

                // 检查坐标是否在原始位图范围内
                let pixel_value = if src_x < bitmap.width().try_into().unwrap()
                    && src_y < bitmap.rows().try_into().unwrap()
                {
                    // 获取原始位图中的像素值
                    let row_start = src_y as usize * bitmap.pitch() as usize;
                    let pixel_index = row_start + src_x as usize;
                    if pixel_index < bitmap.buffer().len() {
                        bitmap.buffer()[pixel_index]
                    } else {
                        0
                    }
                } else {
                    0
                };

                // 转换为黑白像素（阈值设为128）
                if pixel_value > 128 {
                    let byte_index = (y * bytes_per_row + x / 8) as usize;
                    let bit_offset = 7 - (x % 8);
                    char_data[byte_index] |= 1 << bit_offset;
                }
            }
        }
    }

    Ok(result)
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

// 生成适配embedded-graphics的代码
fn generate_embedded_code(
    hitokotos: &Vec<(u32, Vec<Hitokoto>)>,
    categories: &[HitokotoCategory],
    all_chars: &[char],
    glyph_data: &[u8],
    str_refs: &[(usize, usize, usize, usize, usize, usize)],
    global_char_indices: &[usize],
    font_size: u32,
) -> Result<()> {
    let output_dir = Path::new(OUTPUT_DIR);
    fs::create_dir_all(output_dir).context("创建输出目录失败")?;
    let output_path = output_dir.join("hitokoto_data.rs");

    let mut code = String::new();

    // 添加必要的导入
    code.push_str("// 自动生成的代码，请勿手动修改\n");
    code.push_str("#![allow(dead_code)]\n\n");
    code.push_str("use embedded_graphics::mono_font::{MonoFont, MonoFontData};\n\n");

    fn char_literal(c: char) -> String {
        match c {
            '\\' => "'\\\\'".to_string(),
            '\'' => "'\\''".to_string(),
            '\n' => "'\\n'".to_string(),
            '\r' => "'\\r'".to_string(),
            '\t' => "'\\t'".to_string(),
            c if c.is_control() || (c as u32) > 0x7E => {
                format!("'\\u{{{:04X}}}'", c as u32)
            }
            c => format!("'{}'", c),
        }
    }

    // 生成字符数组，每行20个字符（对特殊字符进行转义）
    {
        let mut chars_section = String::new();
        chars_section.push_str("const CHARS: &[char] = &[\n");

        for (i, c) in all_chars.iter().enumerate() {
            if i % 20 == 0 {
                chars_section.push_str("    ");
            }
            let lit = char_literal(*c);
            chars_section.push_str(&format!("{}, ", lit));
            if i % 20 == 19 || i + 1 == all_chars.len() {
                // 去掉行尾多余空格后换行
                if chars_section.ends_with(", ") {
                    let len = chars_section.len();
                    chars_section.truncate(len - 1); // 保留最后的逗号
                }
                chars_section.push_str("\n");
            }
        }

        chars_section.push_str("];\n\n");
        code.push_str(&chars_section);
    }

    // 生成字符索引映射，每行20个索引
    {
        let mut idx_section = String::new();
        idx_section.push_str("const CHAR_INDICES: &[usize] = &[\n");

        for (i, idx) in global_char_indices.iter().enumerate() {
            if i % 20 == 0 {
                idx_section.push_str("    ");
            }
            idx_section.push_str(&format!("{}, ", idx));
            if i % 20 == 19 || i + 1 == global_char_indices.len() {
                if idx_section.ends_with(", ") {
                    let len = idx_section.len();
                    idx_section.truncate(len - 2); // 移除最后一个逗号和空格
                }
                idx_section.push_str(",\n");
            }
        }

        idx_section.push_str("];\n\n");
        let idx_section = idx_section.replace("##", "#\\#");
        code.push_str(&idx_section);
    }

    // 生成字符串引用
    code.push_str("const STRING_REFS: &[(usize, usize, usize, usize, usize, usize)] = &[\n");
    for &(from_start, from_len, from_who_start, from_who_len, content_start, content_len) in
        str_refs
    {
        code.push_str(&format!(
            "    ({}, {}, {}, {}, {}, {}),\n",
            from_start, from_len, from_who_start, from_who_len, content_start, content_len
        ));
    }
    code.push_str("];\n\n");

    // 生成分类数据
    code.push_str("pub struct HitokotoCategory {\n");
    code.push_str("    pub id: u32,\n");
    code.push_str("    pub name: &'static str,\n");
    code.push_str("    pub key: &'static str,\n");
    code.push_str("}\n\n");

    code.push_str("pub const CATEGORIES: &[HitokotoCategory] = &[\n");
    for category in categories {
        code.push_str(&format!(
            "    HitokotoCategory {{ id: {}, name: \"{}\", key: \"{}\" }},\n",
            category.id, category.name, category.key
        ));
    }
    code.push_str("];\n\n");

    // 生成格言数据结构
    code.push_str("pub struct Hitokoto {\n");
    code.push_str("    pub category_id: u32,\n");
    code.push_str("    pub from_indices: (usize, usize),\n"); // (start, length)
    code.push_str("    pub from_who_indices: (usize, usize),\n"); // (start, length)
    code.push_str("    pub content_indices: (usize, usize),\n"); // (start, length)
    code.push_str("}\n\n");

    // 生成格言数据
    code.push_str("pub const HITOKOTOS: &[Hitokoto] = &[\n");
    let mut hitokoto_index = 0;
    for (category_id, hitokoto_list) in hitokotos {
        for _ in hitokoto_list {
            let (from_start, from_len, from_who_start, from_who_len, content_start, content_len) =
                str_refs[hitokoto_index];
            code.push_str(&format!(
                "    Hitokoto {{\n        category_id: {},\n        from_indices: ({}, {}),\n        from_who_indices: ({}, {}),\n        content_indices: ({}, {}),\n    }},\n",
                category_id, from_start, from_len, from_who_start, from_who_len, content_start, content_len
            ));
            hitokoto_index += 1;
        }
    }
    code.push_str("];\n\n");

    // 生成字体数据
    code.push_str("const FONT_DATA: &[u8] = &[\n");
    for chunk in glyph_data.chunks(16) {
        code.push_str("    ");
        for byte in chunk {
            code.push_str(&format!("0x{:02X}, ", byte));
        }
        code.push_str("\n");
    }
    code.push_str("];\n\n");

    // 计算字符宽度（半宽或全宽）
    code.push_str("const fn is_half_width_char(c: char) -> bool {\n");
    code.push_str("    c.is_ascii() || matches!(c, '，' | '。' | '！' | '？' | '、')\n");
    code.push_str("}\n\n");

    code.push_str("const fn char_width(c: char) -> u32 {\n");
    code.push_str("    if is_half_width_char(c) {\n");
    code.push_str(&format!("        {}\n", font_size / 2));
    code.push_str("    } else {\n");
    code.push_str(&format!("        {}\n", font_size));
    code.push_str("    }\n");
    code.push_str("}\n\n");

    // 生成字符宽度数据
    code.push_str("const CHAR_WIDTHS: &[u32] = &[\n");
    for &c in all_chars {
        let width = if c.is_ascii() || matches!(c, '，' | '。' | '！' | '？' | '、') {
            font_size / 2
        } else {
            font_size
        };
        let lit = char_literal(c);
        code.push_str(&format!("    {}, // '{}'\n", width, lit));
    }
    code.push_str("];\n\n");

    // 创建适用于embedded-graphics的MonoFont
    code.push_str("#[rustfmt::skip]\n");
    code.push_str("pub const MONO_FONT: MonoFont = MonoFont {\n");
    code.push_str(&format!(
        "    character_size: embedded_graphics::geometry::Size::new({}, {}),\n",
        font_size, font_size
    ));
    code.push_str("    character_spacing: 0,\n");
    code.push_str("    baseline: 12,\n");
    code.push_str("    underlined_addition: 0,\n");
    code.push_str("    strikethrough_addition: 0,\n");
    code.push_str("    data: MonoFontData {\n");
    code.push_str("        ascii_bitmap: FONT_DATA,\n");
    code.push_str("        glyphs: &[],\n"); // 对于非ASCII字符，我们需要特殊的处理
    code.push_str("    },\n");
    code.push_str("    character_widths: &CHAR_WIDTHS,\n");
    code.push_str("};\n\n");

    // 添加辅助函数
    code.push_str("/// 根据索引获取格言\n");
    code.push_str("pub fn get_hitokoto(index: usize) -> Option<&'static Hitokoto> {\n");
    code.push_str("    HITOKOTOS.get(index)\n");
    code.push_str("}\n\n");

    code.push_str("/// 获取格言总数\n");
    code.push_str("pub fn hitokoto_count() -> usize {\n");
    code.push_str("    HITOKOTOS.len()\n");
    code.push_str("}\n\n");

    code.push_str("/// 根据索引获取分类\n");
    code.push_str("pub fn get_category(index: usize) -> Option<&'static HitokotoCategory> {\n");
    code.push_str("    CATEGORIES.get(index)\n");
    code.push_str("}\n\n");

    code.push_str("/// 获取分类总数\n");
    code.push_str("pub fn category_count() -> usize {\n");
    code.push_str("    CATEGORIES.len()\n");
    code.push_str("}\n\n");

    code.push_str("/// 根据字符获取索引\n");
    code.push_str("pub fn char_to_index(c: char) -> Option<usize> {\n");
    code.push_str("    CHARS.iter().position(|&ch| ch == c)\n");
    code.push_str("}\n\n");

    code.push_str("/// 根据索引获取字符\n");
    code.push_str("pub fn index_to_char(index: usize) -> Option<char> {\n");
    code.push_str("    CHARS.get(index).copied()\n");
    code.push_str("}\n\n");

    // 写入文件
    fs::write(&output_path, code)
        .with_context(|| format!("写入生成代码失败: {}", output_path.display()))?;

    Ok(())
}
