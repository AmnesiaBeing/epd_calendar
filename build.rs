// 1. 删除未使用的PathBuf导入
use std::path::Path;
// 2. 保留必要导入，删除未使用的Font（font-rs的parse返回具体类型，无需显式导入Font）
use anyhow::{Context, Result, anyhow};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use unicode_normalization::UnicodeNormalization;

// 3. 字符串引用辅助结构体
#[derive(Debug, Clone)]
pub struct CategoryWithEnumName {
    pub id: u32,
    pub name: String,
    pub key: String,
    pub enum_name: String, // 改用String存储，避免静态引用生命周期问题
}

#[derive(Debug, Clone, Copy)]
struct StrRef {
    start: usize,
    len: usize,
}

// -------------------------- 配置参数 --------------------------
const FONT_SIZE: u32 = 16;
const OUTPUT_DIR: &str = "src/generated";
const SENTENCES_DIR: &str = "../sentences-bundle/sentences";
const CATEGORIES_PATH: &str = "../sentences-bundle/categories.json";
const FONT_PATH: &str = "assets/SimSun.ttf";

// -------------------------- JSON解析结构体 --------------------------
#[derive(Debug, Deserialize)]
struct RawHitokoto {
    hitokoto: String,
    r#type: String,
    from: String,
    from_who: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawCategory {
    id: u32,
    name: String,
    desc: String,
    key: String,
}

#[derive(Debug, Clone)]
struct ProcessedHitokoto {
    type_enum_name: String, // 存储枚举名称的String，避免引用生命周期问题
    from: String,
    from_who: String,
    content: String,
}

// -------------------------- 主函数 --------------------------
fn main() -> Result<()> {
    // 1. 解析分类信息（返回全局类型CategoryWithEnumName）
    let categories = parse_categories()?;
    // 建立key到枚举名称的映射（临时借用&str，不存储静态引用）
    let key_to_enum_name: BTreeMap<&str, &str> = categories
        .iter()
        .map(|c| (c.key.as_str(), c.enum_name.as_str()))
        .collect();

    // 2. 创建输出目录
    let output_dir = Path::new(OUTPUT_DIR);
    fs::create_dir_all(output_dir).context("创建输出目录失败")?;

    // 3. 读取并解析所有JSON文件（存储枚举名称String）
    let hitokotos = parse_all_json_files(&key_to_enum_name)?;

    // 4. 收集所有字符并过滤
    let (all_chars, char_to_utf16) = collect_and_validate_chars(&hitokotos)?;

    // 5. 生成字体点阵（修复FontError错误）
    let font_bitmaps = generate_font_bitmaps(&all_chars, FONT_SIZE)?;

    // 6. 生成字符串索引映射（修复所有权转移问题）
    let (str_refs, global_char_indices) = generate_string_refs(&hitokotos, &char_to_utf16)?;

    // 7. 生成Rust代码
    generate_rust_code(
        &hitokotos,
        &categories,
        &all_chars,
        &char_to_utf16,
        &font_bitmaps,
        &str_refs,
        &global_char_indices,
        output_dir,
    )?;

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed={}", SENTENCES_DIR);
    println!("cargo:rerun-if-changed={}", CATEGORIES_PATH);
    println!("cargo:rerun-if-changed={}", FONT_PATH);
    Ok(())
}

// -------------------------- 解析分类文件（修复类型和生命周期） --------------------------
fn parse_categories() -> Result<Vec<CategoryWithEnumName>> {
    // 读取分类文件
    let content = fs::read_to_string(CATEGORIES_PATH)
        .with_context(|| format!("读取分类文件失败: {}", CATEGORIES_PATH))?;
    let raw_categories: Vec<RawCategory> =
        serde_json::from_str(&content).context("解析categories.json失败")?;

    // 生成分类信息（全局类型CategoryWithEnumName）
    let categories = raw_categories
        .into_iter()
        .map(|c| {
            // 从desc提取枚举名称（如"Anime - 动画" → "Anime"）
            let enum_name = c
                .desc
                .splitn(2, " - ")
                .next()
                .unwrap_or(&c.name)
                .replace(" ", "")
                .trim()
                .to_string();
            // 首字母大写（PascalCase）
            let enum_name = if let Some(first) = enum_name.chars().next() {
                let mut chars = enum_name.chars();
                chars.next();
                first.to_uppercase().chain(chars).collect()
            } else {
                c.name.replace(" ", "").to_string()
            };
            CategoryWithEnumName {
                id: c.id,
                name: c.name,
                key: c.key,
                enum_name, // 存储为String，避免静态引用
            }
        })
        .collect();

    Ok(categories)
}

// -------------------------- 解析一言JSON（修复枚举名称存储） --------------------------
fn parse_all_json_files(key_to_enum_name: &BTreeMap<&str, &str>) -> Result<Vec<ProcessedHitokoto>> {
    let mut hitokotos = Vec::new();
    let sentences_path = Path::new(SENTENCES_DIR);

    for entry in walkdir::WalkDir::new(sentences_path)
        .min_depth(1)
        .max_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            let content = fs::read_to_string(path)
                .with_context(|| format!("读取JSON文件失败: {}", path.display()))?;

            let raw_items: Vec<RawHitokoto> = serde_json::from_str(&content)
                .with_context(|| format!("解析JSON失败: {}", path.display()))?;

            for item in raw_items {
                // 获取枚举名称（借用临时&str，转为String存储）
                let enum_name = key_to_enum_name
                    .get(item.r#type.as_str())
                    .with_context(|| format!("未知的type: {}", item.r#type))?
                    .to_string();

                hitokotos.push(ProcessedHitokoto {
                    type_enum_name: enum_name,
                    from: item.from,
                    from_who: item.from_who.unwrap_or_else(|| "佚名".to_string()),
                    content: item.hitokoto,
                });
            }
        }
    }

    Ok(hitokotos)
}

// -------------------------- 收集字符（无修改） --------------------------
fn collect_and_validate_chars(
    hitokotos: &[ProcessedHitokoto],
) -> Result<(Vec<char>, HashMap<char, u16>)> {
    let mut char_set = BTreeSet::new();

    for h in hitokotos {
        // 遍历每个字段的字符，新增控制字符过滤
        for c in h.from.nfc() {
            if is_control_char(c) {
                eprintln!("警告：过滤控制字符 [U+{:04X}]（来自from字段）", c as u32);
                continue;
            }
            char_set.insert(c);
        }
        for c in h.from_who.nfc() {
            if is_control_char(c) {
                eprintln!(
                    "警告：过滤控制字符 [U+{:04X}]（来自from_who字段）",
                    c as u32
                );
                continue;
            }
            char_set.insert(c);
        }
        for c in h.content.nfc() {
            if is_control_char(c) {
                eprintln!("警告：过滤控制字符 [U+{:04X}]（来自content字段）", c as u32);
                continue;
            }
            char_set.insert(c);
        }
    }

    // 过滤UTF-16 BMP外字符（原有逻辑不变）
    let mut all_chars = Vec::new();
    let mut char_to_utf16 = HashMap::new();

    for c in char_set {
        let code = c as u32;
        if code > 0xFFFF {
            eprintln!("警告: 字符 '{}' (U+{:04X}) 超出UTF-16范围，已过滤", c, code);
            continue;
        }
        all_chars.push(c);
        char_to_utf16.insert(c, code as u16);
    }

    Ok((all_chars, char_to_utf16))
}

// 新增：判断是否为控制字符（Unicode 控制字符范围）
fn is_control_char(c: char) -> bool {
    let code = c as u32;
    // 控制字符范围：0x0000-0x001F（C0控制符）、0x007F（删除符）
    code <= 0x001F || code == 0x007F
}

// -------------------------- 生成字体点阵（修复FontError和API） --------------------------
fn generate_font_bitmaps(chars: &[char], size: u32) -> Result<Vec<Vec<u8>>> {
    use font_rs::font::{FontError, GlyphBitmap, parse};

    // 读取字体文件
    let font_data = fs::read(FONT_PATH).context("读取字体文件失败")?;
    let font = parse(&font_data)
        .map_err(|e: FontError| anyhow!("解析字体文件失败（font-rs）: {:?}", e))?;

    let mut bitmaps = Vec::with_capacity(chars.len());
    // 计算单个字符的点阵字节数（如16x16 → 32字节）
    let bytes_per_row = (size as usize + 7) / 8;
    let single_bitmap_size = bytes_per_row * size as usize;

    for (char_idx, &c) in chars.iter().enumerate() {
        let code = c as u32;
        let display = if c == ' ' {
            "[空格]".to_string()
        } else {
            c.to_string()
        }; // 空格特殊标记
        eprintln!(
            "正在处理字符 [{}]（索引：{}，Unicode：U+{:04X}）",
            display, char_idx, code
        );

        // 1. 获取Glyph ID（容错：捕获恐慌）
        let glyph_id = match std::panic::catch_unwind(|| font.lookup_glyph_id(code)) {
            Ok(Some(id)) => id,
            Ok(None) => {
                eprintln!("警告：字体中缺少字符 [U+{:04X}]，已生成空点阵", code);
                // 生成空点阵（全0）
                bitmaps.push(vec![0u8; single_bitmap_size]);
                continue;
            }
            Err(_) => {
                eprintln!("警告：处理字符 [U+{:04X}] 时溢出，已生成空点阵", code);
                bitmaps.push(vec![0u8; single_bitmap_size]);
                continue;
            }
        };

        // 2. 渲染点阵（核心修复：空格字符容错）
        let glyph_bitmap = match std::panic::catch_unwind(|| font.render_glyph(glyph_id, size)) {
            Ok(Some(bmp)) => bmp,
            Ok(None) => {
                if code == 0x0020 {
                    // 特殊处理：空格字符渲染失败
                    eprintln!("警告：空格字符（U+0020）渲染失败，已生成空点阵");
                    bitmaps.push(vec![0u8; single_bitmap_size]);
                    continue;
                } else {
                    eprintln!("警告：渲染字符 [U+{:04X}] 失败，已生成空点阵", code);
                    bitmaps.push(vec![0u8; single_bitmap_size]);
                    continue;
                }
            }
            Err(_) => {
                eprintln!("警告：渲染字符 [U+{:04X}] 时溢出，已生成空点阵", code);
                bitmaps.push(vec![0u8; single_bitmap_size]);
                continue;
            }
        };

        // 3. 居中对齐 + 二值化（原有逻辑不变）
        let target_width = size as usize;
        let target_height = size as usize;
        let mut target_bitmap = vec![0u8; single_bitmap_size];

        let offset_x = (target_width as i32 - glyph_bitmap.width as i32) / 2;
        let offset_y = (target_height as i32 - glyph_bitmap.height as i32) / 2;

        for (y, row) in glyph_bitmap
            .data
            .chunks(glyph_bitmap.width as usize)
            .enumerate()
        {
            for (x, &gray) in row.iter().enumerate() {
                let target_x = x as i32 + offset_x;
                let target_y = y as i32 + offset_y;

                if target_x >= 0
                    && target_x < target_width as i32
                    && target_y >= 0
                    && target_y < target_height as i32
                {
                    let tx = target_x as usize;
                    let ty = target_y as usize;

                    if gray > 127 {
                        let byte_idx = ty * bytes_per_row + (tx / 8);
                        let bit_idx = 7 - (tx % 8);
                        target_bitmap[byte_idx] |= 1 << bit_idx;
                    }
                }
            }
        }

        bitmaps.push(target_bitmap);
    }

    Ok(bitmaps)
}

// -------------------------- 生成字符串索引（修复所有权转移） --------------------------
fn generate_string_refs(
    hitokotos: &[ProcessedHitokoto],
    char_to_utf16: &HashMap<char, u16>,
) -> Result<(Vec<(StrRef, StrRef, StrRef)>, Vec<u16>)> {
    let mut global_char_indices = Vec::new();
    let mut str_refs = Vec::with_capacity(hitokotos.len());

    for h in hitokotos {
        // 1. 生成索引序列
        let from_indices: Vec<u16> = h
            .from
            .nfc()
            .map(|c| *char_to_utf16.get(&c).unwrap())
            .collect();
        let from_who_indices: Vec<u16> = h
            .from_who
            .nfc()
            .map(|c| *char_to_utf16.get(&c).unwrap())
            .collect();
        let content_indices: Vec<u16> = h
            .content
            .nfc()
            .map(|c| *char_to_utf16.get(&c).unwrap())
            .collect();

        // 2. 先计算长度（修复：所有权转移前获取长度）
        let from_len = from_indices.len();
        let from_who_len = from_who_indices.len();
        let content_len = content_indices.len();

        // 3. 再执行extend（转移所有权）
        let from_start = global_char_indices.len();
        global_char_indices.extend(from_indices);

        let from_who_start = global_char_indices.len();
        global_char_indices.extend(from_who_indices);

        let content_start = global_char_indices.len();
        global_char_indices.extend(content_indices);

        // 4. 记录StrRef
        str_refs.push((
            StrRef {
                start: from_start,
                len: from_len,
            },
            StrRef {
                start: from_who_start,
                len: from_who_len,
            },
            StrRef {
                start: content_start,
                len: content_len,
            },
        ));
    }

    Ok((str_refs, global_char_indices))
}

// -------------------------- 生成Rust代码（无修改，仅适配String枚举名称） --------------------------
fn generate_rust_code(
    hitokotos: &[ProcessedHitokoto],
    categories: &[CategoryWithEnumName],
    all_chars: &[char],
    char_to_utf16: &HashMap<char, u16>,
    font_bitmaps: &[Vec<u8>],
    str_refs: &[(StrRef, StrRef, StrRef)],
    global_char_indices: &[u16],
    output_dir: &Path,
) -> Result<()> {
    // 生成UTF-16有序数组
    let mut utf16_chars: Vec<u16> = all_chars.iter().map(|c| char_to_utf16[c]).collect();
    utf16_chars.sort_unstable();

    let mut code = String::new();

    // 1. 生成HitokotoType枚举
    code.push_str(
        r#"
//! 自动生成的一言数据和字库
//! 由build.rs生成，请勿手动修改

/// 一言类型枚举（对应categories.json中的分类）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HitokotoType {
"#,
    );
    for c in categories {
        code.push_str(&format!(
            "    /// {}\n    {},\n",
            c.name,
            c.enum_name // 从String取&str
        ));
    }
    code.push_str("}\n\n");

    // 2. 字符串引用结构体
    code.push_str(
        r#"/// 字符串引用：表示字符串在全局字符索引数组中的范围
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StrRef {
    /// 起始索引
    pub start: usize,
    /// 字符长度
    pub len: usize,
}

/// 一言数据结构体
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hitokoto {
    /// 类型（枚举）
    pub type_: HitokotoType,
    /// 来源
    pub from: StrRef,
    /// 来源作者（默认为"佚名"）
    pub from_who: StrRef,
    /// 内容
    pub content: StrRef,
}

"#,
    );

    // 3. 所有字符的UTF-16编码
    code.push_str("/// 所有字符的UTF-16编码（已排序，用于二分查找）\n");
    code.push_str("pub const ALL_CHARS_UTF16: &[u16] = &[\n");
    for &utf16 in &utf16_chars {
        code.push_str(&format!("    0x{:04X},\n", utf16));
    }
    code.push_str("];\n\n");

    // 4. 字体点阵数据
    code.push_str(&format!(
        "/// 字体点阵数据（{}x{}像素，二值化）\n",
        FONT_SIZE, FONT_SIZE
    ));
    code.push_str("pub const FONT_BITMAPS: &[&[u8]] = &[\n");
    for bitmap in font_bitmaps {
        code.push_str("    &[");
        for (i, &byte) in bitmap.iter().enumerate() {
            if i > 0 {
                code.push_str(", ");
            }
            code.push_str(&format!("0x{:02X}", byte));
        }
        code.push_str("],\n");
    }
    code.push_str("];\n\n");

    // 5. 全局字符索引数组
    code.push_str("/// 所有字符串的字符索引序列（from、from_who、content）\n");
    code.push_str("pub const GLOBAL_CHAR_INDICES: &[u16] = &[\n");
    for (i, &idx) in global_char_indices.iter().enumerate() {
        if i % 16 == 0 {
            code.push_str("    ");
        }
        code.push_str(&format!("0x{:04X}, ", idx));
        if (i + 1) % 16 == 0 {
            code.push_str("\n");
        }
    }
    code.push_str("\n];\n\n");

    // 6. 一言数据数组
    code.push_str("/// 所有一言数据\n");
    code.push_str("pub const ALL_HITOKOTOS: &[Hitokoto] = &[\n");
    for (i, (from_ref, from_who_ref, content_ref)) in str_refs.iter().enumerate() {
        let h = &hitokotos[i];
        code.push_str(&format!(
            "    Hitokoto {{\n        type_: HitokotoType::{},\n        from: StrRef {{ start: {}, len: {} }},\n        from_who: StrRef {{ start: {}, len: {} }},\n        content: StrRef {{ start: {}, len: {} }},\n    }},\n",
            h.type_enum_name, // 直接使用String的&str
            from_ref.start, from_ref.len,
            from_who_ref.start, from_who_ref.len,
            content_ref.start, content_ref.len
        ));
    }
    code.push_str("];\n\n");

    // 7. 二分查找函数
    code.push_str(
        r#"/// 二分查找字符的UTF-16编码在字库中的索引
pub fn char_to_index(c: u16) -> Option<usize> {
    let mut low = 0;
    let mut high = ALL_CHARS_UTF16.len();
    
    while low < high {
        let mid = (low + high) / 2;
        match ALL_CHARS_UTF16[mid].cmp(&c) {
            std::cmp::Ordering::Less => low = mid + 1,
            std::cmp::Ordering::Greater => high = mid,
            std::cmp::Ordering::Equal => return Some(mid),
        }
    }
    None
}

/// 通过StrRef获取对应的UTF-16字符序列
pub fn str_ref_to_utf16(s: StrRef) -> &'static [u16] {
    &GLOBAL_CHAR_INDICES[s.start..s.start + s.len]
}
"#,
    );

    // 写入文件
    let output_path = output_dir.join("hitokoto_data.rs");
    fs::write(&output_path, code)
        .with_context(|| format!("写入生成代码失败: {}", output_path.display()))?;

    Ok(())
}
