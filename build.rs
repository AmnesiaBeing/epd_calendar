use anyhow::{Context, Result, anyhow};
use freetype::Library;
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use unicode_normalization::UnicodeNormalization;

// 字体大小，这里只制作FONT_SIZE*FONT_SIZE的中文等宽字体，以及FONT_SIZE/2*FONT_SIZE的英文等宽字体
const FONT_SIZE: u32 = 16;

// 相关目录
const OUTPUT_DIR: &str = "src/app";
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

    // 3. 收集并过滤字符（ASCII、控制字符、非BMP字符）
    let all_chars: Vec<Hitokoto> = hitokotos.clone().into_iter().flat_map(|(_, v)| v).collect();
    let all_chars = collect_and_validate_chars(&all_chars)?;
    eprintln!("共收集有效字符：{}个", all_chars.len());

    // 4. 分离全角和半角字符
    let (full_width_chars, half_width_chars) = separate_full_half_width_chars(&all_chars);
    eprintln!("全角字符：{}个", full_width_chars.len());
    eprintln!("半角字符：{}个", half_width_chars.len());

    // 5. 补充ASCII可见字符到半角字符集
    let half_width_chars = add_ascii_chars(half_width_chars);
    eprintln!("补充ASCII后半角字符：{}个", half_width_chars.len());

    // 6. 确保"佚名"两个字在全角字符集中
    let full_width_chars = ensure_yiming_chars(full_width_chars);

    // 7. 渲染字体点阵
    let (full_width_glyph_data, full_width_char_mapping) =
        render_full_width_font(&full_width_chars)?;
    let (half_width_glyph_data, half_width_char_mapping) =
        render_half_width_font(&half_width_chars)?;

    // 8. 生成格言数据（去重存储作者和来源，None的作者统一为"佚名"）
    generate_hitokoto_data(&hitokotos)?;

    // 9. 生成字体文件
    generate_font_files(
        &full_width_glyph_data,
        &full_width_char_mapping,
        &half_width_glyph_data,
        &half_width_char_mapping,
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

// 收集并过滤字符（ASCII、控制字符、非BMP字符）
fn collect_and_validate_chars(hitokotos: &[Hitokoto]) -> Result<Vec<char>> {
    let mut char_set = BTreeSet::new();

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

    // 过滤不可见字符
    let filtered_chars: Vec<char> = char_set
        .into_iter()
        .filter(|&c| !is_invisible_char(c))
        .collect();

    Ok(filtered_chars)
}

// 判断是否为不可见字符
fn is_invisible_char(c: char) -> bool {
    match c as u32 {
        // 空格、换行、制表符等
        0x20 | 0x09 | 0x0A | 0x0D => true,
        // 不换行空格、软连字符、表意空格
        0xA0 | 0xAD | 0x3000 => true,
        // 其他控制字符
        n if n <= 0x1F || (n >= 0x7F && n <= 0x9F) => true,
        _ => false,
    }
}

// 分离全角和半角字符
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

// 确保"佚名"两个字在全角字符集中
fn ensure_yiming_chars(mut full_width_chars: Vec<char>) -> Vec<char> {
    let mut char_set: BTreeSet<char> = full_width_chars.into_iter().collect();

    // 添加"佚名"两个字
    char_set.insert('佚');
    char_set.insert('名');

    char_set.into_iter().collect()
}

// 判断是否为半角字符
fn if_half_width_char(c: char) -> bool {
    // ASCII字符（除了控制字符）
    if c.is_ascii() && !c.is_ascii_control() {
        return true;
    }

    // 常见的中文半角标点符号
    r#"，。！？；：""''（）【】《》""''""''"#.to_string().find(c).is_some()
}

// 补充ASCII可见字符到半角字符集
fn add_ascii_chars(existing_chars: Vec<char>) -> Vec<char> {
    let mut all_chars: BTreeSet<char> = existing_chars.into_iter().collect();

    // 添加所有ASCII可见字符 (0x21-0x7E)
    for code in 0x21..=0x7E {
        if let Some(c) = std::char::from_u32(code) {
            all_chars.insert(c);
        }
    }

    all_chars.into_iter().collect()
}

// 渲染全角字体（FONT_SIZE × FONT_SIZE）
fn render_full_width_font(chars: &[char]) -> Result<(Vec<u8>, BTreeMap<char, u32>)> {
    let lib = Library::init().map_err(|e| anyhow!("初始化freetype库失败: {}", e))?;
    let face = lib
        .new_face(FONT_PATH, 0)
        .map_err(|e| anyhow!("加载字体文件失败 '{}': {}", FONT_PATH, e))?;

    face.set_pixel_sizes(0, FONT_SIZE)
        .map_err(|e| anyhow!("设置字体大小失败: {}", e))?;

    let mut result = Vec::new();
    let mut char_mapping = BTreeMap::new();

    // 计算每个全角字符需要的字节数
    let bytes_per_row = (FONT_SIZE + 7) / 8;
    let char_data_size = (bytes_per_row * FONT_SIZE) as usize;

    for &c in chars {
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

        let bitmap = face.glyph().bitmap();
        let bitmap_width: u32 = bitmap.width().try_into().unwrap();
        let bitmap_rows: u32 = bitmap.rows().try_into().unwrap();

        // 记录字符映射
        char_mapping.insert(c, (result.len() / char_data_size) as u32);

        // 扩展结果向量以容纳新字符
        let current_len = result.len();
        result.resize(current_len + char_data_size, 0);

        let char_data = &mut result[current_len..current_len + char_data_size];

        // 计算缩放比例
        let scale_x = bitmap_width as f32 / FONT_SIZE as f32;
        let scale_y = bitmap_rows as f32 / FONT_SIZE as f32;

        // 将位图数据转换为指定大小的黑白数据
        for y in 0..FONT_SIZE {
            for x in 0..FONT_SIZE {
                let src_x = (x as f32 * scale_x) as u32;
                let src_y = (y as f32 * scale_y) as u32;

                let pixel_value = if src_x < bitmap_width && src_y < bitmap_rows {
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

                if pixel_value > 128 {
                    let byte_index = (y * bytes_per_row + x / 8) as usize;
                    let bit_offset = 7 - (x % 8);
                    char_data[byte_index] |= 1 << bit_offset;
                }
            }
        }
    }

    Ok((result, char_mapping))
}

// 渲染半角字体（FONT_SIZE/2 × FONT_SIZE）
fn render_half_width_font(chars: &[char]) -> Result<(Vec<u8>, BTreeMap<char, u32>)> {
    let lib = Library::init().map_err(|e| anyhow!("初始化freetype库失败: {}", e))?;
    let face = lib
        .new_face(FONT_PATH, 0)
        .map_err(|e| anyhow!("加载字体文件失败 '{}': {}", FONT_PATH, e))?;

    face.set_pixel_sizes(0, FONT_SIZE)
        .map_err(|e| anyhow!("设置字体大小失败: {}", e))?;

    let mut result = Vec::new();
    let mut char_mapping = BTreeMap::new();

    // 计算每个半角字符需要的字节数
    let target_width = FONT_SIZE / 2;
    let bytes_per_row = (target_width + 7) / 8;
    let char_data_size = (bytes_per_row * FONT_SIZE) as usize;

    for &c in chars {
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

        let bitmap = face.glyph().bitmap();
        let bitmap_width: u32 = bitmap.width().try_into().unwrap();
        let bitmap_rows: u32 = bitmap.rows().try_into().unwrap();

        // 记录字符映射
        char_mapping.insert(c, (result.len() / char_data_size) as u32);

        // 扩展结果向量以容纳新字符
        let current_len = result.len();
        result.resize(current_len + char_data_size, 0);

        let char_data = &mut result[current_len..current_len + char_data_size];

        // 计算缩放比例
        let scale_x = bitmap_width as f32 / target_width as f32;
        let scale_y = bitmap_rows as f32 / FONT_SIZE as f32;

        // 将位图数据转换为指定大小的黑白数据
        for y in 0..FONT_SIZE {
            for x in 0..target_width {
                let src_x = (x as f32 * scale_x) as u32;
                let src_y = (y as f32 * scale_y) as u32;

                let pixel_value = if src_x < bitmap_width && src_y < bitmap_rows {
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

                if pixel_value > 128 {
                    let byte_index = (y * bytes_per_row + x / 8) as usize;
                    let bit_offset = 7 - (x % 8);
                    char_data[byte_index] |= 1 << bit_offset;
                }
            }
        }
    }

    Ok((result, char_mapping))
}

// 生成格言数据文件（去重存储作者和来源，None的作者统一为"佚名"）
fn generate_hitokoto_data(hitokotos: &[(u32, Vec<Hitokoto>)]) -> Result<()> {
    let output_path = Path::new(OUTPUT_DIR).join("hitokoto_data.rs");

    // 收集所有不重复的from和from_who字符串
    let mut from_strings = BTreeSet::new();
    let mut from_who_strings = BTreeSet::new();
    let mut all_hitokotos = Vec::new();

    // 确保"佚名"在作者字符串中
    from_who_strings.insert("佚名".to_string());

    for (category_id, hitokoto_list) in hitokotos {
        for hitokoto in hitokoto_list {
            from_strings.insert(hitokoto.from.clone());

            // 对于None的作者，统一使用"佚名"
            if let Some(from_who) = &hitokoto.from_who {
                from_who_strings.insert(from_who.clone());
            }

            all_hitokotos.push((*category_id, hitokoto));
        }
    }

    // 转换为向量并建立索引映射
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

    // 获取"佚名"的索引
    let yiming_index = from_who_index_map["佚名"];

    let mut content = String::new();

    // 添加文件头
    content.push_str("// 自动生成的格言数据文件\n");
    content.push_str("// 不要手动修改此文件\n\n");

    // 生成来源字符串表
    content.push_str("pub const FROM_STRINGS: &[&str] = &[\n");
    for from_str in &from_vec {
        content.push_str(&format!("    \"{}\",\n", escape_string(from_str)));
    }
    content.push_str("];\n\n");

    // 生成作者字符串表
    content.push_str("pub const FROM_WHO_STRINGS: &[&str] = &[\n");
    for from_who_str in &from_who_vec {
        content.push_str(&format!("    \"{}\",\n", escape_string(from_who_str)));
    }
    content.push_str("];\n\n");

    // 定义Hitokoto结构体
    content.push_str("#[derive(Debug, Clone, Copy)]\n");
    content.push_str("pub struct Hitokoto {\n");
    content.push_str("    pub hitokoto: &'static str,\n");
    content.push_str("    pub from: usize,\n");
    content.push_str("    pub from_who: usize,\n"); // 不再是Option，统一使用索引
    content.push_str("    pub category: u32,\n");
    content.push_str("}\n\n");

    // 生成格言数据
    content.push_str("pub const HITOKOTOS: &[Hitokoto] = &[\n");

    for (category_id, hitokoto) in &all_hitokotos {
        let from_index = from_index_map[hitokoto.from.as_str()];

        // 对于None的作者，使用"佚名"的索引
        let from_who_index = if let Some(from_who) = &hitokoto.from_who {
            from_who_index_map[from_who.as_str()]
        } else {
            yiming_index
        };

        content.push_str(&format!("    Hitokoto {{\n"));
        content.push_str(&format!(
            "        hitokoto: \"{}\",\n",
            escape_string(&hitokoto.hitokoto)
        ));
        content.push_str(&format!("        from: {},\n", from_index));
        content.push_str(&format!("        from_who: {},\n", from_who_index));
        content.push_str(&format!("        category: {},\n", category_id));
        content.push_str(&format!("    }},\n"));
    }

    content.push_str("];\n");

    fs::write(&output_path, content)
        .with_context(|| format!("写入格言数据文件失败: {}", output_path.display()))?;

    eprintln!("格言数据文件生成成功: {}", output_path.display());
    eprintln!("来源字符串数量: {}", from_vec.len());
    eprintln!("作者字符串数量: {}", from_who_vec.len());
    eprintln!("格言总数: {}", all_hitokotos.len());

    Ok(())
}

// 生成字体文件
fn generate_font_files(
    full_width_glyph_data: &[u8],
    full_width_char_mapping: &BTreeMap<char, u32>,
    half_width_glyph_data: &[u8],
    half_width_char_mapping: &BTreeMap<char, u32>,
) -> Result<()> {
    // 1. 生成全角字体二进制文件
    let full_width_bin_path = Path::new(OUTPUT_DIR).join("full_width_font.bin");
    fs::write(&full_width_bin_path, full_width_glyph_data).with_context(|| {
        format!(
            "写入全角字体二进制文件失败: {}",
            full_width_bin_path.display()
        )
    })?;
    eprintln!(
        "全角字体二进制文件生成成功: {} ({}字节)",
        full_width_bin_path.display(),
        full_width_glyph_data.len()
    );

    // 2. 生成半角字体二进制文件
    let half_width_bin_path = Path::new(OUTPUT_DIR).join("half_width_font.bin");
    fs::write(&half_width_bin_path, half_width_glyph_data).with_context(|| {
        format!(
            "写入半角字体二进制文件失败: {}",
            half_width_bin_path.display()
        )
    })?;
    eprintln!(
        "半角字体二进制文件生成成功: {} ({}字节)",
        half_width_bin_path.display(),
        half_width_glyph_data.len()
    );

    // 3. 生成合并的字体描述文件
    let fonts_rs_path = Path::new(OUTPUT_DIR).join("hitokoto_fonts.rs");

    let mut content = String::new();

    content.push_str("// 自动生成的字体描述文件\n");
    content.push_str("// 不要手动修改此文件\n\n");

    content.push_str("use embedded_graphics::{\n");
    content.push_str("    image::ImageRaw,\n");
    content.push_str("    mono_font::{DecorationDimensions, MonoFont, mapping::GlyphMapping},\n");
    content.push_str("    pixelcolor::BinaryColor,\n");
    content.push_str("    prelude::Size,\n");
    content.push_str("};\n\n");

    // 定义二分查找的字符映射实现
    content.push_str("struct BinarySearchGlyphMapping {\n");
    content.push_str("    chars: &'static [u16],\n");
    content.push_str("    offsets: &'static [u32],\n");
    content.push_str("}\n\n");

    content.push_str("impl GlyphMapping for BinarySearchGlyphMapping {\n");
    content.push_str("    fn index(&self, c: char) -> usize {\n");
    content.push_str("        let target = c as u16;\n");
    content.push_str("        let mut left = 0;\n");
    content.push_str("        let mut right = self.chars.len();\n");
    content.push_str("        \n");
    content.push_str("        while left < right {\n");
    content.push_str("            let mid = left + (right - left) / 2;\n");
    content.push_str("            if self.chars[mid] < target {\n");
    content.push_str("                left = mid + 1;\n");
    content.push_str("            } else if self.chars[mid] > target {\n");
    content.push_str("                right = mid;\n");
    content.push_str("            } else {\n");
    content.push_str("                return self.offsets[mid] as usize;\n");
    content.push_str("            }\n");
    content.push_str("        }\n");
    content.push_str("        \n");
    content.push_str("        // 如果没有找到，返回0（显示错了就错了）\n");
    content.push_str("        0\n");
    content.push_str("    }\n");
    content.push_str("}\n\n");

    // 生成全角字体字符映射数组
    let full_width_chars: Vec<u16> = full_width_char_mapping.keys().map(|&c| c as u16).collect();
    let full_width_offsets: Vec<u32> = full_width_char_mapping.values().cloned().collect();

    content.push_str("const FULL_WIDTH_CHARS: &[u16] = &[\n");
    for &c in &full_width_chars {
        content.push_str(&format!("    {},\n", c));
    }
    content.push_str("];\n\n");

    content.push_str("const FULL_WIDTH_OFFSETS: &[u32] = &[\n");
    for &offset in &full_width_offsets {
        content.push_str(&format!("    {},\n", offset));
    }
    content.push_str("];\n\n");

    // 生成半角字体字符映射数组
    let half_width_chars: Vec<u16> = half_width_char_mapping.keys().map(|&c| c as u16).collect();
    let half_width_offsets: Vec<u32> = half_width_char_mapping.values().cloned().collect();

    content.push_str("const HALF_WIDTH_CHARS: &[u16] = &[\n");
    for &c in &half_width_chars {
        content.push_str(&format!("    {},\n", c));
    }
    content.push_str("];\n\n");

    content.push_str("const HALF_WIDTH_OFFSETS: &[u32] = &[\n");
    for &offset in &half_width_offsets {
        content.push_str(&format!("    {},\n", offset));
    }
    content.push_str("];\n\n");

    // 创建静态实例
    content.push_str(
        "static FULL_WIDTH_GLYPH_MAPPING: BinarySearchGlyphMapping = BinarySearchGlyphMapping {\n",
    );
    content.push_str("    chars: FULL_WIDTH_CHARS,\n");
    content.push_str("    offsets: FULL_WIDTH_OFFSETS,\n");
    content.push_str("};\n\n");

    content.push_str(
        "static HALF_WIDTH_GLYPH_MAPPING: BinarySearchGlyphMapping = BinarySearchGlyphMapping {\n",
    );
    content.push_str("    chars: HALF_WIDTH_CHARS,\n");
    content.push_str("    offsets: HALF_WIDTH_OFFSETS,\n");
    content.push_str("};\n\n");

    // 生成全角字体常量（优化格式）
    content.push_str(&format!(
        "pub const FULL_WIDTH_FONT: MonoFont<'static> = MonoFont {{\n\
             image: ImageRaw::<BinaryColor>::new(\n\
                 include_bytes!(\"full_width_font.bin\"),\n\
                 {}\n\
             ),\n\
             glyph_mapping: &FULL_WIDTH_GLYPH_MAPPING,\n\
             character_size: Size::new({}, {}),\n\
             character_spacing: 0,\n\
             baseline: {},\n\
             underline: DecorationDimensions::new({} + 2, 1),\n\
             strikethrough: DecorationDimensions::new({} / 2, 1),\n\
        }};\n\n",
        FONT_SIZE, // 图像宽度
        FONT_SIZE, // 字符宽度
        FONT_SIZE, // 字符高度
        FONT_SIZE, // baseline
        FONT_SIZE,
        FONT_SIZE
    ));

    // 生成半角字体常量（优化格式）
    content.push_str(&format!(
        "pub const HALF_WIDTH_FONT: MonoFont<'static> = MonoFont {{\n\
             image: ImageRaw::<BinaryColor>::new(\n\
                 include_bytes!(\"half_width_font.bin\"),\n\
                 {}\n\
             ),\n\
             glyph_mapping: &HALF_WIDTH_GLYPH_MAPPING,\n\
             character_size: Size::new({}, {}),\n\
             character_spacing: 0,\n\
             baseline: {},\n\
             underline: DecorationDimensions::new({} + 2, 1),\n\
             strikethrough: DecorationDimensions::new({} / 2, 1),\n\
        }};\n",
        FONT_SIZE / 2, // 图像宽度
        FONT_SIZE / 2, // 字符宽度
        FONT_SIZE,     // 字符高度
        FONT_SIZE,     // baseline
        FONT_SIZE / 2,
        FONT_SIZE
    ));

    fs::write(&fonts_rs_path, content)
        .with_context(|| format!("写入字体描述文件失败: {}", fonts_rs_path.display()))?;

    eprintln!("字体描述文件生成成功: {}", fonts_rs_path.display());
    Ok(())
}

// 转义字符串中的特殊字符
fn escape_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('\"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}
