//! 字体生成模块 - 支持可配置的字体尺寸
//! 优化：半角字符使用直接索引计算，全角字符使用二分查找

use crate::builder::config::BuildConfig;
use crate::builder::utils::font_renderer::{FontConfig, FontRenderer};
use crate::builder::utils::{self, progress::ProgressTracker};
use anyhow::{Context, Result};
use std::collections::BTreeSet;
use std::fs;

/// 字体尺寸配置
#[derive(Debug, Clone)]
pub struct FontSizeConfig {
    pub name: String,           // 字体名称，如 "Small", "Medium", "Large"
    pub size: u32,              // 字体高度（像素）
    pub full_width_pixels: u32, // 全角字符宽度（像素）
    pub half_width_pixels: u32, // 半角字符宽度（像素）
}

impl FontSizeConfig {
    /// 创建新的字体尺寸配置
    pub fn new(name: &str, size: u32) -> Self {
        Self {
            name: name.to_string(),
            size,
            full_width_pixels: size,     // 全角字符为正方形
            half_width_pixels: size / 2, // 半角字符宽度为高度的一半
        }
    }

    /// 计算每个字符的字节大小
    pub fn glyph_size(&self, is_full_width: bool) -> usize {
        let height = self.size as usize;
        let width = if is_full_width {
            self.full_width_pixels as usize
        } else {
            self.half_width_pixels as usize
        };
        width * height / 8
    }
}

/// 共享字符表
#[derive(Debug)]
pub struct SharedCharset {
    pub full_width: Vec<char>,
    pub half_width: Vec<char>,
    pub half_width_start: u16, // 半角字符起始编码
    pub half_width_end: u16,   // 半角字符结束编码
}

/// 字体位图数据
#[derive(Debug)]
pub struct FontBitmap {
    pub glyph_data: Vec<u8>,
    pub config: FontSizeConfig,
    pub is_full_width: bool,
    pub actual_glyph_size: usize, // 实际计算的每个字符字节数
    pub char_count: usize,        // 字符数量
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

    // 定义字体尺寸配置 - 必须与渲染时使用的字号一致！
    let font_configs = vec![
        FontSizeConfig::new("Small", 16),  // 小号字体 16px
        FontSizeConfig::new("Medium", 24), // 中号字体 24px
        FontSizeConfig::new("Large", 40),  // 大号字体 40px
    ];

    let mut font_bitmaps = Vec::new();

    // 为每个字体配置渲染全角和半角字符
    for (i, font_config) in font_configs.iter().enumerate() {
        progress.update_progress(
            i + 2,
            font_configs.len() + 2,
            &format!("渲染{}字体", font_config.name),
        );

        // 渲染全角字符
        let full_bitmap = render_font_bitmap(
            config,
            &shared_charset.full_width,
            font_config.clone(),
            false,
        )?;

        // 渲染半角字符
        let half_bitmap = render_font_bitmap(
            config,
            &shared_charset.half_width,
            font_config.clone(),
            true,
        )?;

        font_bitmaps.push((full_bitmap, half_bitmap));
    }

    progress.update_progress(
        font_configs.len() + 2,
        font_configs.len() + 2,
        "生成字体文件",
    );
    generate_shared_font_files(config, &shared_charset, &font_configs, &font_bitmaps)?;

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

    Ok(SharedCharset {
        full_width,
        half_width,
        half_width_start: 0x20,
        half_width_end: 0x7E,
    })
}

/// 渲染字体位图数据
fn render_font_bitmap(
    config: &BuildConfig,
    chars: &[char],
    font_config: FontSizeConfig,
    is_half_width: bool,
) -> Result<FontBitmap> {
    let font_render_config = FontConfig {
        font_path: config.font_path.to_string_lossy().to_string(),
        font_size: font_config.size,
        is_half_width,
        chars: chars.to_vec(),
    };

    let render_result = FontRenderer::render_font(&font_render_config)?;

    // 计算实际每个字符的字节数
    let char_count = chars.len();
    let total_bytes = render_result.glyph_data.len();
    let actual_glyph_size = if char_count > 0 {
        total_bytes / char_count
    } else {
        0
    };

    // 计算预期的字符大小
    let expected_glyph_size = font_config.glyph_size(!is_half_width);

    if actual_glyph_size != expected_glyph_size {
        println!(
            "cargo:warning=  警告: {}字体 {}-width 字符大小不匹配: 实际={}, 预期={}",
            font_config.name,
            if is_half_width { "half" } else { "full" },
            actual_glyph_size,
            expected_glyph_size
        );
    }

    Ok(FontBitmap {
        glyph_data: render_result.glyph_data,
        config: font_config.clone(),
        is_full_width: !is_half_width,
        actual_glyph_size: expected_glyph_size,
        char_count,
    })
}

/// 生成共享字符表的字体文件
fn generate_shared_font_files(
    config: &BuildConfig,
    charset: &SharedCharset,
    font_configs: &[FontSizeConfig],
    font_bitmaps: &[(FontBitmap, FontBitmap)],
) -> Result<()> {
    // 1. 生成二进制文件
    for (i, (full_bitmap, half_bitmap)) in font_bitmaps.iter().enumerate() {
        let font_name = &font_configs[i].name.to_lowercase();

        write_font_bitmap(
            full_bitmap,
            &config
                .output_dir
                .join(format!("generated_{}_full_width_font.bin", font_name)),
        )?;

        write_font_bitmap(
            half_bitmap,
            &config
                .output_dir
                .join(format!("generated_{}_half_width_font.bin", font_name)),
        )?;
    }

    // 2. 生成Rust源文件 - 使用实际的字符大小
    generate_fonts_rs(config, charset, font_configs, font_bitmaps)?;

    Ok(())
}

/// 写入位图数据到文件
fn write_font_bitmap(bitmap: &FontBitmap, path: &std::path::Path) -> Result<()> {
    std::fs::write(path, &bitmap.glyph_data)?;

    let width = if bitmap.is_full_width {
        bitmap.config.full_width_pixels
    } else {
        bitmap.config.half_width_pixels
    };

    // 使用实际计算的字符大小
    let glyph_size = bitmap.actual_glyph_size;

    println!(
        "cargo:warning=  生成字体文件: {}, 大小: {}KB (字符数: {}, 每个字符{}字节, {}x{}像素)",
        path.display(),
        bitmap.glyph_data.len() / 1024,
        bitmap.char_count,
        glyph_size,
        width,
        bitmap.config.size
    );

    Ok(())
}

/// 生成字体数据Rust源文件
fn generate_fonts_rs(
    config: &BuildConfig,
    charset: &SharedCharset,
    font_configs: &[FontSizeConfig],
    font_bitmaps: &[(FontBitmap, FontBitmap)],
) -> Result<()> {
    let output_path = config.output_dir.join("generated_fonts.rs");

    let mut content = String::new();

    // 头部注释 - 明确说明实际渲染尺寸
    content.push_str("//! 自动生成的字体数据文件（共享字符表）\n");
    content.push_str("//! 不要手动修改此文件\n\n");

    content.push_str("use embedded_graphics::geometry::Size;\n");

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

    // 半角字符快速索引常量
    content.push_str("/// 半角字符起始编码 (用于快速索引)\n");
    content.push_str(&format!(
        "pub const HALF_WIDTH_START: u16 = 0x{:04X};\n\n",
        charset.half_width_start
    ));

    content.push_str("/// 半角字符结束编码 (用于快速索引)\n");
    content.push_str(&format!(
        "pub const HALF_WIDTH_END: u16 = 0x{:04X};\n\n",
        charset.half_width_end
    ));

    // ========== 2. 字体常量 - 使用实际渲染的尺寸 ==========
    content.push_str("// ==================== 字体常量 ====================\n");
    content.push_str("// 注意: 以下常量基于实际渲染的字体尺寸\n\n");

    // 为每个字体配置生成常量
    for (i, font_config) in font_configs.iter().enumerate() {
        let (full_bitmap, half_bitmap) = &font_bitmaps[i];
        let name_upper = font_config.name.to_uppercase();

        // 全角字体常量 - 使用实际计算的字符大小
        let full_glyph_size = full_bitmap.actual_glyph_size;

        content.push_str(&format!(
            "// {}字体 ({}px) - 全角\n",
            font_config.name, font_config.size
        ));
        content.push_str(&format!(
            "pub const FONT_{}_FULL_WIDTH: u8 = {};\n",
            name_upper, font_config.full_width_pixels
        ));
        content.push_str(&format!(
            "pub const FONT_{}_FULL_HEIGHT: u8 = {};\n",
            name_upper, font_config.size
        ));
        content.push_str(&format!(
            "pub const FONT_{}_FULL_GLYPH_SIZE: usize = {};\n\n",
            name_upper, full_glyph_size
        ));

        // 半角字体常量 - 使用实际计算的字符大小
        let half_glyph_size = half_bitmap.actual_glyph_size;

        content.push_str(&format!(
            "// {}字体 ({}px) - 半角\n",
            font_config.name, font_config.size
        ));
        content.push_str(&format!(
            "pub const FONT_{}_HALF_WIDTH: u8 = {};\n",
            name_upper, font_config.half_width_pixels
        ));
        content.push_str(&format!(
            "pub const FONT_{}_HALF_HEIGHT: u8 = {};\n",
            name_upper, font_config.size
        ));
        content.push_str(&format!(
            "pub const FONT_{}_HALF_GLYPH_SIZE: usize = {};\n\n",
            name_upper, half_glyph_size
        ));
    }

    // ========== 3. 位图数据 ==========
    content.push_str("// ==================== 位图数据 ====================\n\n");

    // 使用 include_bytes! 嵌入二进制数据
    for font_config in font_configs {
        let name_lower = font_config.name.to_lowercase();

        content.push_str(&format!("// {}字体位图\n", font_config.name));
        content.push_str(&format!("pub const {}_FULL_WIDTH_BITMAP: &[u8] = include_bytes!(\"generated_{}_full_width_font.bin\");\n", 
            name_lower.to_uppercase(), name_lower));
        content.push_str(&format!("pub const {}_HALF_WIDTH_BITMAP: &[u8] = include_bytes!(\"generated_{}_half_width_font.bin\");\n\n", 
            name_lower.to_uppercase(), name_lower));
    }

    // ========== 4. 辅助函数 ==========
    content.push_str("// ==================== 辅助函数 ====================\n\n");

    // 二分查找函数（用于全角字符）
    content.push_str("/// 二分查找字符索引（用于全角字符）\n");
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

    // 快速获取半角字符索引
    content.push_str("/// 快速获取半角字符索引（无需二分查找）\n");
    content.push_str("#[inline(always)]\n");
    content.push_str("fn get_half_width_index(c: u16) -> Option<usize> {\n");
    content.push_str(&format!(
        "    if c >= 0x{:04X} && c <= 0x{:04X} {{\n",
        charset.half_width_start, charset.half_width_end
    ));
    content.push_str("        // 直接计算索引：索引 = (字符编码 - 起始编码)\n");
    content.push_str(&format!(
        "        let idx = (c - 0x{:04X}) as usize;\n",
        charset.half_width_start
    ));
    content.push_str("        Some(idx)\n");
    content.push_str("    } else {\n");
    content.push_str("        None\n");
    content.push_str("    }\n");
    content.push_str("}\n\n");

    // 字符宽度类型枚举
    content.push_str("/// 字符宽度类型\n");
    content.push_str("#[derive(Copy, Clone, Debug, PartialEq, Eq)]\n");
    content.push_str("pub enum CharWidth {\n");
    content.push_str("    Full,\n");
    content.push_str("    Half,\n");
    content.push_str("}\n\n");

    // 字体尺寸枚举
    content.push_str("/// 字体尺寸\n");
    content.push_str("#[derive(Copy, Clone, Debug, PartialEq, Eq)]\n");
    content.push_str("pub enum FontSize {\n");
    for font_config in font_configs {
        content.push_str(&format!("    {},\n", font_config.name));
    }
    content.push_str("}\n\n");

    // 为FontSize实现方法
    content.push_str("impl FontSize {\n");
    content.push_str("    /// 根据字符宽度类型获取字形数据\n");
    content.push_str("    /// \n");
    content.push_str("    /// # 参数\n");
    content.push_str("    /// - `c`: 要查找的字符\n");
    content.push_str("    /// - `width_type`: 字符宽度类型\n");
    content.push_str("    /// \n");
    content.push_str("    /// # 返回值\n");
    content.push_str("    /// 如果找到字符，返回字形数据的切片；否则返回None\n");
    content.push_str(
        "    pub fn get_glyph(self, c: char, width_type: CharWidth) -> Option<&'static [u8]> {\n",
    );
    content.push_str("        let target = c as u16;\n");
    content.push_str("        \n");
    content.push_str("        match width_type {\n");
    content.push_str("            CharWidth::Full => {\n");
    content.push_str("                // 全角字符使用二分查找\n");
    content.push_str("                let idx = find_char_index(FULL_WIDTH_CHARS, target)?;\n");
    content.push_str("                match self {\n");

    for font_config in font_configs {
        let name_upper = font_config.name.to_uppercase();

        content.push_str(&format!(
            "                    FontSize::{} => {{\n",
            font_config.name
        ));
        content.push_str(&format!(
            "                        let glyph_size = FONT_{}_FULL_GLYPH_SIZE;\n",
            name_upper
        ));
        content.push_str(&format!(
            "                        let start = idx.checked_mul(glyph_size)?;\n"
        ));
        content.push_str(&format!(
            "                        let end = start.checked_add(glyph_size)?;\n"
        ));
        content.push_str(&format!(
            "                        let bitmap = {}_FULL_WIDTH_BITMAP;\n",
            name_upper
        ));
        content.push_str("                        if end <= bitmap.len() {\n");
        content.push_str(&format!(
            "                            Some(&bitmap[start..end])\n"
        ));
        content.push_str("                        } else {\n");
        content.push_str("                            None\n");
        content.push_str("                        }\n");
        content.push_str("                    }\n");
    }

    content.push_str("                }\n");
    content.push_str("            }\n");
    content.push_str("            CharWidth::Half => {\n");
    content.push_str("                // 半角字符使用快速索引\n");
    content.push_str("                let idx = get_half_width_index(target)?;\n");
    content.push_str("                match self {\n");

    for font_config in font_configs {
        let name_upper = font_config.name.to_uppercase();

        content.push_str(&format!(
            "                    FontSize::{} => {{\n",
            font_config.name
        ));
        content.push_str(&format!(
            "                        let glyph_size = FONT_{}_HALF_GLYPH_SIZE;\n",
            name_upper
        ));
        content.push_str(&format!(
            "                        let start = idx.checked_mul(glyph_size)?;\n"
        ));
        content.push_str(&format!(
            "                        let end = start.checked_add(glyph_size)?;\n"
        ));
        content.push_str(&format!(
            "                        let bitmap = {}_HALF_WIDTH_BITMAP;\n",
            name_upper
        ));
        content.push_str("                        if end <= bitmap.len() {\n");
        content.push_str(&format!(
            "                            Some(&bitmap[start..end])\n"
        ));
        content.push_str("                        } else {\n");
        content.push_str("                            None\n");
        content.push_str("                    }\n");
        content.push_str("                }\n");
    }

    content.push_str("                }\n");
    content.push_str("            }\n");
    content.push_str("        }\n");
    content.push_str("    }\n\n");

    content.push_str("    /// 自动判断字符宽度并获取字形\n");
    content.push_str("    /// \n");
    content.push_str("    /// ASCII字符使用半角宽度，其他字符使用全角宽度\n");
    content.push_str(
        "    pub fn get_glyph_auto(self, c: char) -> (CharWidth, Option< &'static [u8]>) {\n",
    );
    content.push_str("        let width_type = if c.is_ascii() && c.is_ascii_graphic() {\n");
    content.push_str("            CharWidth::Half\n");
    content.push_str("        } else {\n");
    content.push_str("            CharWidth::Full\n");
    content.push_str("        };\n");
    content.push_str("        (width_type, self.get_glyph(c, width_type))\n");
    content.push_str("    }\n\n");

    content.push_str("    /// 获取字体大小\n");
    content.push_str("    pub fn size(&self, width_type: CharWidth) -> Size {\n");
    content.push_str("        match self {\n");
    for font_config in font_configs {
        content.push_str(&format!(
            "            FontSize::{} => match width_type {{
                CharWidth::Full => Size::new({}, {}),
                CharWidth::Half => Size::new({}, {}),
            }},\n",
            font_config.name,
            font_config.full_width_pixels,
            font_config.size,
            font_config.half_width_pixels,
            font_config.size
        ));
    }
    content.push_str("        }\n");
    content.push_str("    }\n");

    content.push_str("    /// 获取字体大小（像素高度）\n");
    content.push_str("    pub fn height(&self) -> u8 {\n");
    content.push_str("        match self {\n");
    for font_config in font_configs {
        content.push_str(&format!(
            "            FontSize::{} => {},\n",
            font_config.name, font_config.size
        ));
    }
    content.push_str("        }\n");
    content.push_str("    }\n");

    content.push_str("}\n\n");

    // 为CharWidth实现方法
    content.push_str("impl CharWidth {\n");
    content.push_str("    /// 根据字符自动判断宽度类型\n");
    content.push_str("    pub fn from_char(c: char) -> Self {\n");
    content.push_str("        if c.is_ascii() && c.is_ascii_graphic() {\n");
    content.push_str("            CharWidth::Half\n");
    content.push_str("        } else {\n");
    content.push_str("            CharWidth::Full\n");
    content.push_str("        }\n");
    content.push_str("    }\n");
    content.push_str("}\n\n");

    // ========== 6. 字体验证函数 ==========
    content.push_str("// ==================== 调试和预览函数 ====================\n\n");

    content.push_str("/// 预览字形数据（将字形以ASCII形式打印到日志）\n");
    content.push_str("/// \n");
    content.push_str("/// # 参数\n");
    content.push_str("/// - `glyph_data`: 字形数据切片\n");
    content.push_str("/// - `width`: 字形宽度（像素）\n");
    content.push_str("/// - `height`: 字形高度（像素）\n");
    content.push_str("/// - `char_info`: 字符信息字符串（用于日志）\n");
    content.push_str(
        "pub fn preview_glyph(glyph_data: &[u8], width: u8, height: u8, char_info: char) {\n",
    );
    content.push_str("    use log::info;\nuse alloc::format;\nuse alloc::string::String;\nuse alloc::vec::Vec;\n");
    content.push_str("    \n");
    content.push_str("    info!(\"预览字符: {}\", char_info);\n");
    content.push_str(
        "    info!(\"字形尺寸: {}x{}, 总字节数: {}\", width, height, glyph_data.len());\n",
    );
    content.push_str("    \n");
    content.push_str("    // 验证数据长度是否匹配\n");
    content.push_str("    let expected_len = width as usize * height as usize / 8;\n");
    content.push_str("    if glyph_data.len() != expected_len {\n");
    content.push_str("        info!(\"警告: 字形数据长度不匹配! 实际: {}, 预期: {}\", glyph_data.len(), expected_len);\n");
    content.push_str("    }\n");
    content.push_str("    \n");
    content.push_str("    // 构建ASCII预览\n");
    content.push_str("    let mut preview = String::new();\n");
    content
        .push_str("    preview.push_str(&format!(\"┌{}┐\\n\", \"─\".repeat(width as usize)));\n");
    content.push_str("    \n");
    content.push_str("    for y in 0..height {\n");
    content.push_str("        preview.push('│');\n");
    content.push_str("        let row_start = (y as usize) * (width as usize) / 8;\n");
    content.push_str("        \n");
    content.push_str("        for x in 0..width {\n");
    content.push_str("            let byte_index = row_start + (x as usize / 8);\n");
    content.push_str("            if byte_index < glyph_data.len() {\n");
    content.push_str("                // MSB优先（最高位对应最左边的像素）\n");
    content.push_str("                let bit_position = 7 - (x % 8);\n");
    content.push_str("                let pixel = (glyph_data[byte_index] >> bit_position) & 1;\n");
    content.push_str("                if pixel == 1 {\n");
    content.push_str("                    preview.push('█');\n");
    content.push_str("                } else {\n");
    content.push_str("                    preview.push(' ');\n");
    content.push_str("                }\n");
    content.push_str("            } else {\n");
    content.push_str("                preview.push('?');\n");
    content.push_str("            }\n");
    content.push_str("        }\n");
    content.push_str("        preview.push_str(\"│\\n\");\n");
    content.push_str("        \n");
    content.push_str("        // 每行也显示字节数据\n");
    content.push_str("        let row_end = row_start + width as usize / 8;\n");
    content.push_str("        if row_end <= glyph_data.len() {\n");
    content.push_str("            let row_bytes = &glyph_data[row_start..row_end];\n");
    content.push_str("            let hex_str: Vec<String> = row_bytes.iter().map(|b| format!(\"{:02X}\", b)).collect();\n");
    content.push_str("            info!(\"行{}字节: {}\", y, hex_str.join(\" \"));\n");
    content.push_str("        }\n");
    content.push_str("    }\n");
    content.push_str("    \n");
    content
        .push_str("    preview.push_str(&format!(\"└{}┘\\n\", \"─\".repeat(width as usize)));\n");
    content.push_str("    info!(\"字形预览:\\n{}\", preview);\n");
    content.push_str("}\n\n");

    // 写入文件
    utils::file_utils::write_string_file(&output_path, &content)?;

    println!(
        "cargo:warning=  生成字体描述文件: {}",
        output_path.display()
    );

    Ok(())
}
