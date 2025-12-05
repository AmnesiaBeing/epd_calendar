//! 通用字体渲染器
#![allow(unused)]

use anyhow::{Result, anyhow};
use freetype::Library;
use freetype::bitmap::PixelMode;
use std::collections::BTreeMap;
use std::time::Instant;

/// 字体渲染配置
#[derive(Debug, Clone)]
pub struct FontConfig {
    pub font_path: String,
    pub font_size: u32,
    pub is_half_width: bool,
    pub chars: Vec<char>,
}

/// 字体渲染结果
pub struct FontRenderResult {
    pub glyph_data: Vec<u8>,
    pub char_mapping: BTreeMap<char, u32>,
    pub char_width: u32,
    pub char_height: u32,
    pub rendered_chars: usize,    // 实际渲染的字符数量
    pub missing_chars: Vec<char>, // 缺失的字符列表
}

/// 通用字体渲染器
pub struct FontRenderer;

impl FontRenderer {
    /// 渲染字体
    pub fn render_font(config: &FontConfig) -> Result<FontRenderResult> {
        let start_time = Instant::now();

        let lib = Library::init().map_err(|e| anyhow!("初始化freetype库失败: {}", e))?;
        let face = lib
            .new_face(&config.font_path, 0)
            .map_err(|e| anyhow!("加载字体文件失败 '{}': {}", config.font_path, e))?;

        face.set_pixel_sizes(0, config.font_size)
            .map_err(|e| anyhow!("设置字体大小失败: {}", e))?;

        let char_width = if config.is_half_width {
            config.font_size / 2
        } else {
            config.font_size
        };
        let char_height = config.font_size;

        let bytes_per_row = (char_width + 7) / 8;
        let char_data_size = (bytes_per_row * char_height) as usize;

        let mut glyph_data = Vec::with_capacity(config.chars.len() * char_data_size);
        let mut char_mapping = BTreeMap::new();
        let mut missing_chars = Vec::new();
        let mut rendered_chars = 0;

        println!(
            "cargo:warning=  开始渲染字体: {} ({}px, {}-width)",
            config.font_path,
            config.font_size,
            if config.is_half_width { "half" } else { "full" }
        );
        println!(
            "cargo:warning=  字符数量: {}, 每字符尺寸: {}x{}, 每字符字节数: {}",
            config.chars.len(),
            char_width,
            char_height,
            char_data_size
        );

        for (index, &c) in config.chars.iter().enumerate() {
            // 检查字体中是否有这个字符
            let Some(glyph_index) = face.get_char_index(c as usize) else {
                missing_chars.push(c);
                println!(
                    "cargo:warning=  警告: 字符 '{}' (0x{:04X}) 在字体中未找到，跳过",
                    c, c as u32
                );
                continue;
            };

            if glyph_index == 0 {
                missing_chars.push(c);
                println!(
                    "cargo:warning=  警告: 字符 '{}' (0x{:04X}) 的字形索引为0，跳过",
                    c, c as u32
                );
                continue;
            }

            // 加载字形
            match face.load_glyph(glyph_index, freetype::face::LoadFlag::RENDER) {
                Ok(_) => (),
                Err(e) => {
                    missing_chars.push(c);
                    println!(
                        "cargo:warning=  警告: 加载字符 '{}' (0x{:04X}) 的字形失败: {}，跳过",
                        c, c as u32, e
                    );
                    continue;
                }
            }

            let glyph = face.glyph();
            let bitmap = glyph.bitmap();

            let bitmap_width = bitmap.width() as u32;
            let bitmap_rows = bitmap.rows() as u32;
            let bitmap_pitch = bitmap.pitch().abs() as u32;

            // 记录字符映射关系
            char_mapping.insert(c, (glyph_data.len() / char_data_size) as u32);

            // 为当前字符分配空间
            let current_len = glyph_data.len();
            glyph_data.resize(current_len + char_data_size, 0);

            let char_data = &mut glyph_data[current_len..current_len + char_data_size];
            char_data.fill(0);

            // 计算居中偏移量
            let x_offset = if bitmap_width < char_width {
                (char_width - bitmap_width) / 2
            } else {
                0
            };

            let y_offset = if bitmap_rows < char_height {
                (char_height - bitmap_rows) / 2
            } else {
                0
            };

            // 复制位图数据
            match bitmap.pixel_mode() {
                Ok(PixelMode::Mono) => {
                    // 单色位图，每个像素用1位表示
                    for y in 0..bitmap_rows {
                        let target_y = y + y_offset;
                        if target_y >= char_height {
                            break;
                        }

                        for x in 0..bitmap_width {
                            let target_x = x + x_offset;
                            if target_x >= char_width {
                                break;
                            }

                            // 计算源位图中的字节和位索引
                            let byte_index = (y * bitmap_pitch + x / 8) as usize;
                            if byte_index >= bitmap.buffer().len() {
                                continue;
                            }

                            let bit_index = 7 - (x % 8);
                            let pixel_value = (bitmap.buffer()[byte_index] >> bit_index) & 1;

                            if pixel_value == 1 {
                                // 目标位图：计算字节和位索引
                                let target_byte_index =
                                    (target_y * bytes_per_row + target_x / 8) as usize;
                                let target_bit_offset = 7 - (target_x % 8);

                                if target_byte_index < char_data.len() {
                                    char_data[target_byte_index] |= 1 << target_bit_offset;
                                }
                            }
                        }
                    }
                }
                Ok(PixelMode::Gray) => {
                    // 灰度位图，每个像素用8位表示
                    let threshold = 128; // 50%阈值，大于阈值的视为黑色

                    for y in 0..bitmap_rows {
                        let target_y = y + y_offset;
                        if target_y >= char_height {
                            break;
                        }

                        for x in 0..bitmap_width {
                            let target_x = x + x_offset;
                            if target_x >= char_width {
                                break;
                            }

                            // 计算源位图中的像素索引
                            let pixel_index = (y * bitmap_pitch + x) as usize;
                            if pixel_index >= bitmap.buffer().len() {
                                continue;
                            }

                            let pixel_value = bitmap.buffer()[pixel_index];

                            // 灰度值大于阈值视为黑色
                            if pixel_value > threshold {
                                // 目标位图：计算字节和位索引
                                let target_byte_index =
                                    (target_y * bytes_per_row + target_x / 8) as usize;
                                let target_bit_offset = 7 - (target_x % 8);

                                if target_byte_index < char_data.len() {
                                    char_data[target_byte_index] |= 1 << target_bit_offset;
                                }
                            }
                        }
                    }
                }
                _ => {
                    println!(
                        "cargo:warning=  警告: 字符 '{}' (0x{:04X}) 使用不支持的像素模式，跳过",
                        c, c as u32
                    );
                    // 移除刚分配的空间
                    glyph_data.truncate(current_len);
                    missing_chars.push(c);
                    continue;
                }
            }

            rendered_chars += 1;

            // 显示进度（每100个字符或最后一批）
            if rendered_chars % 100 == 0 || index == config.chars.len() - 1 {
                let progress = ((index + 1) as f32 / config.chars.len() as f32 * 100.0) as u32;
                println!(
                    "cargo:warning=  字体渲染进度: {}% ({}/{})，已渲染: {}，缺失: {}",
                    progress,
                    index + 1,
                    config.chars.len(),
                    rendered_chars,
                    missing_chars.len()
                );
            }
        }

        let duration = start_time.elapsed();

        println!(
            "cargo:warning=  字体渲染完成，耗时: {:.2}秒",
            duration.as_secs_f32()
        );
        println!(
            "cargo:warning=  统计: 总共{}字符，成功渲染{}，缺失{}",
            config.chars.len(),
            rendered_chars,
            missing_chars.len()
        );

        if !missing_chars.is_empty() {
            // 只显示前20个缺失字符，避免输出太多
            let show_count = missing_chars.len().min(20);
            println!(
                "cargo:warning=  缺失字符 (前{}个): {:?}",
                show_count,
                &missing_chars[..show_count]
            );

            if missing_chars.len() > 20 {
                println!(
                    "cargo:warning=  还有{}个缺失字符未显示",
                    missing_chars.len() - 20
                );
            }
        }

        // 验证数据大小
        let expected_size = rendered_chars * char_data_size;
        if glyph_data.len() != expected_size {
            println!(
                "cargo:warning=  警告: 字形数据大小不匹配! 实际: {}字节, 预期: {}字节 (差: {}字节)",
                glyph_data.len(),
                expected_size,
                glyph_data.len() as isize - expected_size as isize
            );
        }

        Ok(FontRenderResult {
            glyph_data,
            char_mapping,
            char_width,
            char_height,
            rendered_chars,
            missing_chars,
        })
    }

    /// 渲染单个字符用于调试
    pub fn render_single_char(
        font_path: &str,
        font_size: u32,
        c: char,
    ) -> Result<(Vec<u8>, u32, u32)> {
        let lib = Library::init().map_err(|e| anyhow!("初始化freetype库失败: {}", e))?;
        let face = lib
            .new_face(font_path, 0)
            .map_err(|e| anyhow!("加载字体文件失败 '{}': {}", font_path, e))?;

        face.set_pixel_sizes(0, font_size)
            .map_err(|e| anyhow!("设置字体大小失败: {}", e))?;

        // 使用全角宽度
        let char_width = font_size;
        let char_height = font_size;

        let bytes_per_row = (char_width + 7) / 8;
        let char_data_size = (bytes_per_row * char_height) as usize;

        // 获取字形索引
        let Some(glyph_index) = face.get_char_index(c as usize) else {
            return Err(anyhow!("字符 '{}' (0x{:04X}) 在字体中未找到", c, c as u32));
        };

        if glyph_index == 0 {
            return Err(anyhow!("字符 '{}' (0x{:04X}) 的字形索引为0", c, c as u32));
        }

        // 加载字形
        face.load_glyph(glyph_index, freetype::face::LoadFlag::RENDER)
            .map_err(|e| anyhow!("加载字形失败 '{}': {}", c, e))?;

        let glyph = face.glyph();
        let bitmap = glyph.bitmap();

        let bitmap_width = bitmap.width() as u32;
        let bitmap_rows = bitmap.rows() as u32;
        let bitmap_pitch = bitmap.pitch().abs() as u32;

        let mut char_data = vec![0u8; char_data_size];

        // 计算居中偏移量
        let x_offset = if bitmap_width < char_width {
            (char_width - bitmap_width) / 2
        } else {
            0
        };

        let y_offset = if bitmap_rows < char_height {
            (char_height - bitmap_rows) / 2
        } else {
            0
        };

        // 复制位图数据
        match bitmap.pixel_mode() {
            Ok(PixelMode::Mono) => {
                for y in 0..bitmap_rows {
                    let target_y = y + y_offset;
                    if target_y >= char_height {
                        break;
                    }

                    for x in 0..bitmap_width {
                        let target_x = x + x_offset;
                        if target_x >= char_width {
                            break;
                        }

                        let byte_index = (y * bitmap_pitch + x / 8) as usize;
                        if byte_index >= bitmap.buffer().len() {
                            continue;
                        }

                        let bit_index = 7 - (x % 8);
                        let pixel_value = (bitmap.buffer()[byte_index] >> bit_index) & 1;

                        if pixel_value == 1 {
                            let target_byte_index =
                                (target_y * bytes_per_row + target_x / 8) as usize;
                            let target_bit_offset = 7 - (target_x % 8);

                            if target_byte_index < char_data.len() {
                                char_data[target_byte_index] |= 1 << target_bit_offset;
                            }
                        }
                    }
                }
            }
            Ok(PixelMode::Gray) => {
                let threshold = 128;

                for y in 0..bitmap_rows {
                    let target_y = y + y_offset;
                    if target_y >= char_height {
                        break;
                    }

                    for x in 0..bitmap_width {
                        let target_x = x + x_offset;
                        if target_x >= char_width {
                            break;
                        }

                        let pixel_index = (y * bitmap_pitch + x) as usize;
                        if pixel_index >= bitmap.buffer().len() {
                            continue;
                        }

                        let pixel_value = bitmap.buffer()[pixel_index];

                        if pixel_value > threshold {
                            let target_byte_index =
                                (target_y * bytes_per_row + target_x / 8) as usize;
                            let target_bit_offset = 7 - (target_x % 8);

                            if target_byte_index < char_data.len() {
                                char_data[target_byte_index] |= 1 << target_bit_offset;
                            }
                        }
                    }
                }
            }
            _ => {
                return Err(anyhow!("字符 '{}' 使用不支持的像素模式", c));
            }
        }

        Ok((char_data, char_width, char_height))
    }

    /// 检查字体是否包含指定字符
    pub fn check_char_in_font(font_path: &str, c: char) -> Result<bool> {
        let lib = Library::init().map_err(|e| anyhow!("初始化freetype库失败: {}", e))?;
        let face = lib
            .new_face(font_path, 0)
            .map_err(|e| anyhow!("加载字体文件失败 '{}': {}", font_path, e))?;

        let glyph_index = face.get_char_index(c as usize);
        Ok(glyph_index.is_some() && glyph_index.unwrap_or(0) != 0)
    }

    /// 批量检查字符在字体中的存在性
    pub fn check_chars_in_font(font_path: &str, chars: &[char]) -> Result<Vec<(char, bool)>> {
        let lib = Library::init().map_err(|e| anyhow!("初始化freetype库失败: {}", e))?;
        let face = lib
            .new_face(font_path, 0)
            .map_err(|e| anyhow!("加载字体文件失败 '{}': {}", font_path, e))?;

        let mut results = Vec::with_capacity(chars.len());

        for &c in chars {
            let glyph_index = face.get_char_index(c as usize);
            let exists = glyph_index.is_some() && glyph_index.unwrap_or(0) != 0;
            results.push((c, exists));
        }

        Ok(results)
    }

    /// 生成字符存在性报告
    pub fn generate_char_report(font_path: &str, chars: &[char]) -> Result<(Vec<char>, Vec<char>)> {
        let results = Self::check_chars_in_font(font_path, chars)?;

        let mut existing = Vec::new();
        let mut missing = Vec::new();

        for (c, exists) in results {
            if exists {
                existing.push(c);
            } else {
                missing.push(c);
            }
        }

        println!(
            "cargo:warning=  字符报告: 字体 '{}' 包含 {}/{} 个字符",
            font_path,
            existing.len(),
            chars.len()
        );

        if !missing.is_empty() {
            println!("cargo:warning=  缺失字符数量: {}", missing.len());

            let show_count = missing.len().min(10);
            println!(
                "cargo:warning=  缺失字符示例 (前{}个): {:?}",
                show_count,
                &missing[..show_count]
            );
        }

        Ok((existing, missing))
    }
}
