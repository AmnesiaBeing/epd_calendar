//! 通用图标渲染器（优化版）

use anyhow::{Context, Result, anyhow};
use std::path::Path;

/// 图标渲染配置
#[derive(Debug, Clone)]
pub struct IconConfig {
    pub target_width: u32,
    pub target_height: u32,
    pub svg_path: String,
}

/// 图标渲染结果
pub struct IconRenderResult {
    pub bitmap_data: Vec<u8>,
}

/// 通用图标渲染器
pub struct IconRenderer;

impl IconRenderer {
    /// 渲染SVG图标（返回带尺寸信息）
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
        let mut pixmap = resvg::tiny_skia::Pixmap::new(config.target_width, config.target_height)
            .ok_or_else(|| anyhow!("创建像素图失败"))?;

        // 使用 resvg::Tree 进行渲染
        let rtree = resvg::Tree::from_usvg(&tree);

        // 计算缩放比例
        let svg_size = rtree.view_box.rect.size();
        let scale_x = config.target_width as f32 / svg_size.width();
        let scale_y = config.target_height as f32 / svg_size.height();
        let scale = scale_x.min(scale_y); // 保持宽高比

        // 计算居中偏移
        let offset_x = (config.target_width as f32 - svg_size.width() * scale) / 2.0;
        let offset_y = (config.target_height as f32 - svg_size.height() * scale) / 2.0;

        // 创建缩放和平移变换
        let transform = resvg::tiny_skia::Transform::from_scale(scale, scale)
            .post_translate(offset_x, offset_y);

        // 渲染到 pixmap
        rtree.render(transform, &mut pixmap.as_mut());

        // 转换为 1-bit 位图
        let bitmap_data = Self::convert_to_1bit(&pixmap, config.target_width, config.target_height);

        Ok(IconRenderResult { bitmap_data })
    }

    /// 将 RGBA 像素图转换为 1-bit 位图（优化版）
    fn convert_to_1bit(pixmap: &resvg::tiny_skia::Pixmap, width: u32, height: u32) -> Vec<u8> {
        let width = width as usize;
        let height = height as usize;

        // 计算每行所需的字节数
        let bytes_per_row = (width + 7) / 8;
        let mut result = vec![0u8; bytes_per_row * height];

        // 使用行主序遍历，提高缓存局部性
        for y in 0..height {
            let row_offset = y * bytes_per_row;
            for x in 0..width {
                if let Some(pixel) = pixmap.pixel(x as u32, y as u32) {
                    // 优化判断逻辑
                    let alpha = pixel.alpha() as f32 / 255.0;
                    let is_visible = alpha > 0.1; // 降低阈值，捕捉更多细节

                    if is_visible {
                        let byte_index = row_offset + x / 8;
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
}
