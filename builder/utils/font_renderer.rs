//! 字体渲染器 - 渲染字符二值位图并记录度量参数
//! 存储：字形二值图像、偏移量、宽度、高度、BearingXY

use anyhow::{Result, anyhow};
use freetype::Library;
use freetype::bitmap::PixelMode;
use std::collections::BTreeMap;

/// 单个字符的字形度量参数（存储偏移 + 几何参数）
#[derive(Debug, Clone, Copy)]
pub struct GlyphMetrics {
    /// 字符在bin文件中的起始偏移（字节）
    pub offset: u32,
    /// 字符位图宽度（像素）
    pub width: u32,
    /// 字符位图高度（像素）
    pub height: u32,
    /// 水平偏移（BearingX）：字符位图相对基线的X偏移（像素）
    pub bearing_x: i32,
    /// 垂直偏移（BearingY）：字符位图相对基线的Y偏移（像素）
    pub bearing_y: i32,
    /// 水平Advance（AdvanceX）：字符渲染后的X轴移动距离（像素）
    pub advance_x: i32,
}

impl Default for GlyphMetrics {
    fn default() -> Self {
        Self {
            offset: 0,
            width: 0,
            height: 0,
            bearing_x: 0,
            bearing_y: 0,
            advance_x: 0,
        }
    }
}

/// 字体渲染配置
#[derive(Debug, Clone)]
pub struct FontConfig {
    pub font_path: String,
    pub font_size: u32,
    pub chars: Vec<char>,
}

/// 字体渲染结果
pub struct FontRenderResult {
    pub glyph_data: Vec<u8>,               // 所有字符的二值图像数据（按顺序拼接）
    pub char_mapping: BTreeMap<char, u32>, // 字符 -> 在glyph_data中的起始偏移（字节）
    pub glyph_metrics_map: BTreeMap<char, GlyphMetrics>, // 每个字符的度量参数
    pub rendered_chars: usize,             // 成功渲染的字符数
    pub missing_chars: Vec<char>,          // 缺失字符
}

/// 字体渲染器
pub struct FontRenderer;

impl FontRenderer {
    /// 渲染字体 - 生成字符二值位图和度量参数
    pub fn render_font(config: &FontConfig) -> Result<FontRenderResult> {
        // let start_time = std::time::Instant::now();

        // 1. 初始化FreeType
        let lib = Library::init().map_err(|e| anyhow!("初始化freetype库失败: {}", e))?;

        // 2. 准备字符集
        let all_chars = config.chars.clone();

        // 3. 渲染所有字符
        let mut glyph_data = Vec::new();
        let mut char_mapping = BTreeMap::new();
        let mut glyph_metrics_map = BTreeMap::new();
        let mut missing_chars = Vec::new();
        let mut rendered_chars = 0;

        for &c in &all_chars {
            let codepoint = c as u32;

            // 为每个字符重新创建Face对象，避免状态污染
            let face = lib
                .new_face(&config.font_path, 0)
                .map_err(|e| anyhow!("加载字体文件失败 '{}': {}", config.font_path, e))?;

            // 设置字体大小（像素）
            face.set_pixel_sizes(0, config.font_size)
                .map_err(|e| anyhow!("设置字体大小失败: {}", e))?;

            // 加载字形索引
            let Some(glyph_index) = face.get_char_index(codepoint as usize) else {
                missing_chars.push(c);
                continue;
            };

            if glyph_index == 0 {
                missing_chars.push(c);
                continue;
            }

            // 加载并渲染字形（生成二值位图）
            face.load_glyph(glyph_index, freetype::face::LoadFlag::RENDER)
                .map_err(|e| anyhow!("加载字形失败 '{}': {}", c, e))?;

            let glyph = face.glyph();
            let bitmap = glyph.bitmap();
            let glyph_metrics = glyph.metrics();

            // 提取原始度量参数（转换为像素，FreeType的度量单位是1/64像素）
            let bearing_x_px = (glyph_metrics.horiBearingX >> 6) as i32;
            let bearing_y_px = (glyph_metrics.horiBearingY >> 6) as i32;
            let bitmap_width_px = bitmap.width() as u32;
            let bitmap_height_px = bitmap.rows() as u32;
            let advance_x_px = (glyph_metrics.horiAdvance >> 6) as i32;

            // 跳过空位图
            if bitmap_width_px == 0 || bitmap_height_px == 0 {
                missing_chars.push(c);
                continue;
            }

            // 记录字符在bin中的起始偏移
            let char_start_offset = glyph_data.len() as u32;
            char_mapping.insert(c, char_start_offset);

            // 计算字符位图所需字节数（二值图像：每8像素1字节）
            let bytes_per_row = (bitmap_width_px + 7) / 8;
            let char_data_size = (bytes_per_row * bitmap_height_px) as usize;

            // 为当前字符分配空间
            let current_len = glyph_data.len();
            glyph_data.resize(current_len + char_data_size, 0);
            let char_data = &mut glyph_data[current_len..current_len + char_data_size];

            // 复制位图数据（转换为二值图像）
            let copy_result = match bitmap.pixel_mode() {
                Ok(PixelMode::Mono) => {
                    Self::copy_mono_bitmap(&bitmap, char_data, bytes_per_row);
                    true
                }
                Ok(PixelMode::Gray) => {
                    Self::copy_gray_bitmap(&bitmap, char_data, bytes_per_row);
                    true
                }
                _ => false,
            };

            if !copy_result {
                // 渲染失败，回滚
                glyph_data.truncate(current_len);
                char_mapping.remove(&c);
                missing_chars.push(c);
                continue;
            }

            // 存储当前字符的度量参数
            glyph_metrics_map.insert(
                c,
                GlyphMetrics {
                    offset: char_start_offset,
                    width: bitmap_width_px,
                    height: bitmap_height_px,
                    bearing_x: bearing_x_px,
                    bearing_y: bearing_y_px,
                    advance_x: advance_x_px,
                },
            );

            rendered_chars += 1;

            // 每1000个字符打印进度
            // if rendered_chars % 1000 == 0 {
            //     println!(
            //         "cargo:warning=  渲染进度: {}/{} ({}%)",
            //         rendered_chars,
            //         all_chars.len(),
            //         (rendered_chars * 1000) / all_chars.len()
            //     );
            // }
        }

        // let duration = start_time.elapsed();

        // 打印统计信息
        // println!(
        //     "cargo:warning=  字体渲染完成，耗时: {:.2}秒",
        //     duration.as_secs_f32()
        // );
        // println!(
        //     "cargo:warning=  统计: 总共{}字符，成功渲染{}，缺失{}",
        //     all_chars.len(),
        //     rendered_chars,
        //     missing_chars.len()
        // );
        // println!(
        //     "cargo:warning=  生成位图数据大小: {}KB",
        //     glyph_data.len() / 1024
        // );

        Ok(FontRenderResult {
            glyph_data,
            char_mapping,
            glyph_metrics_map,
            rendered_chars,
            missing_chars,
        })
    }

    /// 复制单色位图（Mono）到目标缓冲区
    fn copy_mono_bitmap(bitmap: &freetype::Bitmap, target: &mut [u8], target_bytes_per_row: u32) {
        let bitmap_width = bitmap.width() as u32;
        let bitmap_height = bitmap.rows() as u32;
        let bitmap_pitch = bitmap.pitch().abs() as u32;
        let buffer = bitmap.buffer();

        for y in 0..bitmap_height {
            for x in 0..bitmap_width {
                // 计算源位图的字节索引和位索引
                let src_byte_idx = (y * bitmap_pitch + x / 8) as usize;
                let src_bit_idx = 7 - (x % 8); // Mono位图是MSB优先

                if src_byte_idx >= buffer.len() {
                    continue;
                }

                // 读取源像素值
                let pixel = (buffer[src_byte_idx] >> src_bit_idx) & 1;
                if pixel == 0 {
                    continue; // 跳过空白像素
                }

                // 计算目标缓冲区的字节索引和位索引
                let target_byte_idx = (y * target_bytes_per_row + x / 8) as usize;
                let target_bit_idx = 7 - (x % 8); // 目标也是MSB优先

                if target_byte_idx < target.len() {
                    target[target_byte_idx] |= 1 << target_bit_idx;
                }
            }
        }
    }

    /// 复制灰度位图（Gray）并转换为二值图像
    fn copy_gray_bitmap(bitmap: &freetype::Bitmap, target: &mut [u8], target_bytes_per_row: u32) {
        let bitmap_width = bitmap.width() as u32;
        let bitmap_height = bitmap.rows() as u32;
        let bitmap_pitch = bitmap.pitch().abs() as u32;
        let buffer = bitmap.buffer();

        // 二值化阈值（128/255）
        const THRESHOLD: u8 = 128;

        for y in 0..bitmap_height {
            for x in 0..bitmap_width {
                // 计算源位图的像素索引
                let src_pixel_idx = (y * bitmap_pitch + x) as usize;
                if src_pixel_idx >= buffer.len() {
                    continue;
                }

                // 灰度值转二值
                let pixel_value = buffer[src_pixel_idx];
                if pixel_value <= THRESHOLD {
                    continue; // 低于阈值视为空白
                }

                // 计算目标缓冲区的字节索引和位索引
                let target_byte_idx = (y * target_bytes_per_row + x / 8) as usize;
                let target_bit_idx = 7 - (x % 8); // MSB优先

                if target_byte_idx < target.len() {
                    target[target_byte_idx] |= 1 << target_bit_idx;
                }
            }
        }
    }

    /// 测试字体度量信息（调试用）
    #[allow(unused)]
    pub fn test_font_metrics(font_path: &str, font_size: u32) -> Result<()> {
        let test_chars = vec!['0', '1', '2', '中', '文', '测', '试', ',', '。', '!', '？'];

        let lib = Library::init()?;
        let face = lib.new_face(font_path, 0)?;
        face.set_pixel_sizes(0, font_size)?;

        println!("字体: {}, 大小: {}px", font_path, font_size);
        println!("\n字符度量信息:");
        println!("字符 | 宽度 | 高度 | BearingX | BearingY");
        println!("-----|------|------|----------|----------");

        for &c in &test_chars {
            let Some(glyph_index) = face.get_char_index(c as usize) else {
                println!("{:^3} | 缺失 | 缺失 | 缺失     | 缺失", c);
                continue;
            };

            face.load_glyph(glyph_index, freetype::face::LoadFlag::DEFAULT)?;
            let glyph = face.glyph();
            let metrics = glyph.metrics();
            let bitmap = glyph.bitmap();

            println!(
                "{:^3} | {:^4} | {:^4} | {:^8} | {:^8}",
                c,
                bitmap.width(),
                bitmap.rows(),
                metrics.horiBearingX >> 6,
                metrics.horiBearingY >> 6
            );
        }

        Ok(())
    }

    /// 渲染单个字符用于预览（调试用）
    #[allow(unused)]
    pub fn preview_single_char(
        font_path: &str,
        font_size: u32,
        c: char,
    ) -> Result<(Vec<u8>, GlyphMetrics)> {
        let lib = Library::init()?;
        let face = lib.new_face(font_path, 0)?;
        face.set_pixel_sizes(0, font_size)?;

        let Some(glyph_index) = face.get_char_index(c as usize) else {
            return Err(anyhow!("字符 '{}' 在字体中未找到", c));
        };

        if glyph_index == 0 {
            return Err(anyhow!("字符 '{}' 的字形索引为0", c));
        }

        face.load_glyph(glyph_index, freetype::face::LoadFlag::RENDER)?;
        let glyph = face.glyph();
        let bitmap = glyph.bitmap();
        let metrics = glyph.metrics();

        // 提取度量参数
        let bearing_x = (metrics.horiBearingX >> 6) as i32;
        let bearing_y = (metrics.horiBearingY >> 6) as i32;
        let width = bitmap.width() as u32;
        let height = bitmap.rows() as u32;
        let advance_x = (metrics.horiAdvance >> 6) as i32;

        // 生成二值位图
        let bytes_per_row = (width + 7) / 8;
        let char_data_size = (bytes_per_row * height) as usize;
        let mut char_data = vec![0u8; char_data_size];

        match bitmap.pixel_mode() {
            Ok(PixelMode::Mono) => {
                Self::copy_mono_bitmap(&bitmap, &mut char_data, bytes_per_row);
            }
            Ok(PixelMode::Gray) => {
                Self::copy_gray_bitmap(&bitmap, &mut char_data, bytes_per_row);
            }
            _ => {
                return Err(anyhow!("不支持的像素模式"));
            }
        }

        Ok((
            char_data,
            GlyphMetrics {
                offset: 0,
                width,
                height,
                bearing_x,
                bearing_y,
                advance_x,
            },
        ))
    }
}
