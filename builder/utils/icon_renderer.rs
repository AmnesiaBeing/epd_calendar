//! 通用图标渲染器

use anyhow::{Context, Result, anyhow};
use std::path::Path;

/// 图标渲染配置
#[derive(Debug, Clone)]
pub struct IconConfig {
    pub icon_size: u32,
    pub svg_path: String,
}

/// 图标渲染结果
pub struct IconRenderResult {
    pub bitmap_data: Vec<u8>,
}

/// 通用图标渲染器
pub struct IconRenderer;

impl IconRenderer {
    /// 渲染SVG图标
    pub fn render_svg_icon(config: &IconConfig) -> Result<IconRenderResult> {
        let svg_path = Path::new(&config.svg_path);

        if !svg_path.exists() {
            return Err(anyhow!("SVG文件不存在: {}", config.svg_path));
        }

        let svg_data = std::fs::read(svg_path)
            .with_context(|| format!("读取SVG文件失败: {}", config.svg_path))?;

        // 解析 SVG
        use usvg::TreeParsing;
        let options = usvg::Options::default();
        let tree = usvg::Tree::from_data(&svg_data, &options)
            .map_err(|e| anyhow!("解析SVG失败 {}: {}", config.svg_path, e))?;

        // 创建像素图
        let mut pixmap = resvg::tiny_skia::Pixmap::new(config.icon_size, config.icon_size)
            .ok_or_else(|| anyhow!("创建像素图失败"))?;

        // 使用 resvg::Tree 进行渲染
        let rtree = resvg::Tree::from_usvg(&tree);

        // 计算缩放比例
        let svg_size = rtree.view_box.rect.size();
        let scale_x = config.icon_size as f32 / svg_size.width();
        let scale_y = config.icon_size as f32 / svg_size.height();
        let scale = scale_x.min(scale_y); // 保持宽高比

        // 计算居中偏移
        let offset_x = (config.icon_size as f32 - svg_size.width() * scale) / 2.0;
        let offset_y = (config.icon_size as f32 - svg_size.height() * scale) / 2.0;

        // 创建缩放和平移变换
        let transform = resvg::tiny_skia::Transform::from_scale(scale, scale)
            .post_translate(offset_x, offset_y);

        // 渲染到 pixmap
        rtree.render(transform, &mut pixmap.as_mut());

        // 转换为 1-bit 位图
        let bitmap_data = Self::convert_to_1bit(&pixmap, config.icon_size);

        Ok(IconRenderResult { bitmap_data })
    }

    /// 将 RGBA 像素图转换为 1-bit 位图
    fn convert_to_1bit(pixmap: &resvg::tiny_skia::Pixmap, icon_size: u32) -> Vec<u8> {
        let width = icon_size as usize;
        let height = icon_size as usize;

        // 计算每行所需的字节数
        let bytes_per_row = (width + 7) / 8;
        let mut result = vec![0u8; bytes_per_row * height];

        for y in 0..height {
            for x in 0..width {
                if let Some(pixel) = pixmap.pixel(x as u32, y as u32) {
                    // 阈值处理，转换为黑白
                    let alpha = pixel.alpha() as f32 / 255.0;
                    let is_black = alpha > 0.5
                        && (pixel.red() < 250 || pixel.green() < 250 || pixel.blue() < 250);

                    if is_black {
                        let byte_index = y * bytes_per_row + x / 8;
                        let bit_offset = 7 - (x % 8); // MSB 优先

                        if byte_index < result.len() {
                            result[byte_index] |= 1 << bit_offset;
                        }
                    }
                }
            }
        }

        result
    }

    /// 预览图标
    #[allow(dead_code)]
    pub fn preview_icon(result: &IconRenderResult, name: &str, icon_size: u32) {
        let width = icon_size as usize;
        let height = icon_size as usize;
        let bytes_per_row = (width + 7) / 8;

        println!(
            "cargo:warning=  图标预览 '{}' ({}x{}):",
            name, width, height
        );

        for y in 0..height {
            let mut line = String::new();
            for x in 0..width {
                let byte_index = y * bytes_per_row + x / 8;
                let bit_offset = 7 - (x % 8); // MSB 优先

                let pixel = if byte_index < result.bitmap_data.len() {
                    (result.bitmap_data[byte_index] >> bit_offset) & 1
                } else {
                    0
                };

                // 使用不同的字符来创建更好的视觉效果
                line.push(if pixel == 1 { '█' } else { ' ' });
            }
            println!("cargo:warning=  {}", line);
        }
        println!("cargo:warning=");
    }

    /// 预览多个图标（并排显示）
    #[allow(dead_code)]
    pub fn preview_icons_multiple(results: &[(&str, &IconRenderResult)], icon_size: u32) {
        let width = icon_size as usize;
        let height = icon_size as usize;
        let bytes_per_row = (width + 7) / 8;

        println!("cargo:warning=  多图标预览 (共{}个):", results.len());

        // 打印标题行
        let titles: Vec<String> = results
            .iter()
            .map(|(name, _)| format!("{:^width$}", name, width = width))
            .collect();
        println!("cargo:warning=  {}", titles.join(" "));

        // 逐行渲染所有图标
        for y in 0..height {
            let mut line = String::new();
            for (_, result) in results {
                for x in 0..width {
                    let byte_index = y * bytes_per_row + x / 8;
                    let bit_offset = 7 - (x % 8);

                    let pixel = if byte_index < result.bitmap_data.len() {
                        (result.bitmap_data[byte_index] >> bit_offset) & 1
                    } else {
                        0
                    };

                    line.push(if pixel == 1 { '█' } else { ' ' });
                }
                line.push(' '); // 图标之间的间隔
            }
            println!("cargo:warning=  {}", line);
        }
        println!("cargo:warning=");
    }
}
