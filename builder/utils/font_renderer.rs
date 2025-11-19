//! 通用字体渲染器

use anyhow::{Result, anyhow};
use freetype::Library;
use freetype::bitmap::PixelMode;
use std::collections::BTreeMap;

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
}

/// 通用字体渲染器
pub struct FontRenderer;

impl FontRenderer {
    /// 渲染字体
    pub fn render_font(config: &FontConfig) -> Result<FontRenderResult> {
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

        let mut glyph_data = Vec::new();
        let mut char_mapping = BTreeMap::new();

        for (index, &c) in config.chars.iter().enumerate() {
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

            char_mapping.insert(c, (glyph_data.len() / char_data_size) as u32);

            let current_len = glyph_data.len();
            glyph_data.resize(current_len + char_data_size, 0);

            let char_data = &mut glyph_data[current_len..current_len + char_data_size];
            char_data.fill(0);

            let x_offset = if (bitmap_width as u32) < char_width {
                (char_width - bitmap_width as u32) / 2
            } else {
                0
            };

            let y_offset = if (bitmap_rows as u32) < char_height {
                (char_height - bitmap_rows as u32) / 2
            } else {
                0
            };

            for y in 0..(bitmap_rows as u32) {
                let target_y = y + y_offset;
                if target_y >= char_height {
                    break;
                }

                for x in 0..(bitmap_width as u32) {
                    let target_x = x + x_offset;
                    if target_x >= char_width {
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

            // 显示进度（每100个字符）
            if index % 100 == 0 {
                println!(
                    "cargo:warning=  字体渲染进度: {}/{}",
                    index,
                    config.chars.len()
                );
            }
        }

        Ok(FontRenderResult {
            glyph_data,
            char_mapping,
            char_width,
            char_height,
        })
    }

    /// 预览字符串
    pub fn preview_string(result: &FontRenderResult, s: &str, font_type: &str) {
        let bytes_per_row = (result.char_width + 7) / 8;
        let char_data_size = (bytes_per_row * result.char_height) as usize;

        println!("cargo:warning=  {}字体预览 '{}':", font_type, s);

        for row in 0..result.char_height {
            let mut line = String::new();
            for c in s.chars() {
                if let Some(&char_index) = result.char_mapping.get(&c) {
                    let start = (char_index as usize) * char_data_size;
                    let char_data = &result.glyph_data[start..start + char_data_size];

                    for x in 0..result.char_width {
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
                    line.push_str(&" ".repeat(result.char_width as usize));
                }
            }
            println!("cargo:warning=  {}", line);
        }
        println!("cargo:warning=");
    }
}
