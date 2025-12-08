//! 布局渲染引擎模块
//! 加载编译期生成的布局二进制，反序列化，处理条件过滤和占位符替换，调用文本/图标/图形渲染工具绘制到4色墨水屏

use alloc::format;
use core::str::FromStr;
use embedded_graphics::{
    Drawable,
    draw_target::DrawTarget,
    geometry::{Point, Size},
    image::Image,
    mono_font::MonoTextStyle,
    pixelcolor::BinaryColor,
    primitives::{Circle, Line as GfxLine, PrimitiveStyle, Rectangle as GfxRectangle},
};
use heapless::{String, Vec};
use postcard::from_bytes;
use serde::Deserialize;

// 定义固定大小的字符串类型
type String32 = String<32>;
type String64 = String<64>;
type String128 = String<128>;

use crate::{
    assets::{
        generated_fonts::{Font, FontCollection},
        generated_icons::{ICON_DATA, Icon, IconId},
        generated_layouts::MAIN_LAYOUT_BIN,
    },
    common::error::{AppError, Result},
    kernel::data::{registry::DataSourceRegistry, types::DynamicValue},
    kernel::driver::display::DisplayDriver,
};

// ==================== 布局元素定义（与builder保持一致） ====================

// 从编译期代码导入布局元素定义
// 注意：这里需要确保与编译期生成的布局二进制中的类型完全一致

/// 重要程度枚举
#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
pub enum Importance {
    Normal,   // Black
    Warning,  // Yellow
    Critical, // Red
}

impl FromStr for Importance {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "" | "normal" | "black" => Ok(Importance::Normal),
            "warning" | "yellow" => Ok(Importance::Warning),
            "critical" | "red" => Ok(Importance::Critical),
            _ => Err(AppError::RenderError),
        }
    }
}

/// 边框定义
#[derive(Debug, Clone, Deserialize)]
pub struct Border {
    pub top: u16,
    pub right: u16,
    pub bottom: u16,
    pub left: u16,
}

impl Default for Border {
    fn default() -> Self {
        Self {
            top: 0,
            right: 0,
            bottom: 0,
            left: 0,
        }
    }
}

/// 文本对齐方式
#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
pub enum TextAlignment {
    Left,
    Center,
    Right,
}

impl FromStr for TextAlignment {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "left" => Ok(TextAlignment::Left),
            "center" => Ok(TextAlignment::Center),
            "right" => Ok(TextAlignment::Right),
            _ => Err(AppError::RenderError),
        }
    }
}

/// 垂直对齐方式
#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
pub enum VerticalAlignment {
    Top,
    Center,
    Bottom,
}

impl FromStr for VerticalAlignment {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "top" => Ok(VerticalAlignment::Top),
            "center" => Ok(VerticalAlignment::Center),
            "bottom" => Ok(VerticalAlignment::Bottom),
            _ => Err(AppError::RenderError),
        }
    }
}

/// 容器布局方向
#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
pub enum ContainerDirection {
    Horizontal,
    Vertical,
}

impl FromStr for ContainerDirection {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "horizontal" => Ok(ContainerDirection::Horizontal),
            "vertical" => Ok(ContainerDirection::Vertical),
            _ => Err(AppError::RenderError),
        }
    }
}

/// 子元素布局配置
#[derive(Debug, Clone, Deserialize)]
pub struct ChildLayout {
    pub node: LayoutNode,
    pub weight: Option<f32>, // 权重，用于比例布局
    pub is_absolute: bool,   // 是否为绝对定位
}

/// 布局节点枚举
#[derive(Debug, Clone, Deserialize)]
pub enum LayoutNode {
    Container(Container),
    Text(Text),
    Icon(IconElement),
    Line(LineElement),
    Rectangle(RectangleElement),
    Circle(CircleElement),
}

/// 容器元素
#[derive(Debug, Clone, Deserialize)]
pub struct Container {
    pub id: String32,
    pub rect: [u16; 4],
    pub children: Vec<ChildLayout, 8>,
    pub condition: Option<String128>,
    pub direction: ContainerDirection,
    pub alignment: TextAlignment,
    pub vertical_alignment: VerticalAlignment,
    pub border: Border,
}

/// 文本元素
#[derive(Debug, Clone, Deserialize)]
pub struct Text {
    pub id: String32,
    pub rect: [u16; 4],
    pub content: String128,
    pub font_size: String32, // 字体大小配置名称，如 "small", "medium", "large"
    pub alignment: TextAlignment,
    pub vertical_alignment: VerticalAlignment,
    pub max_width: Option<u16>,
    pub max_lines: Option<u8>,
}

/// 图标元素
#[derive(Debug, Clone, Deserialize)]
pub struct IconElement {
    pub id: String32,
    pub rect: [u16; 4],
    pub icon_id: String128, // 可以是静态ID或格式化字符串
    pub importance: Option<Importance>,
}

/// 线条元素
#[derive(Debug, Clone, Deserialize)]
pub struct LineElement {
    pub id: String32,
    pub start: [u16; 2],
    pub end: [u16; 2],
    pub thickness: u16,
    pub importance: Option<Importance>,
}

/// 矩形元素
#[derive(Debug, Clone, Deserialize)]
pub struct RectangleElement {
    pub id: String32,
    pub rect: [u16; 4],
    pub fill_importance: Option<Importance>,
    pub stroke_importance: Option<Importance>,
    pub stroke_thickness: u16,
}

/// 圆形元素
#[derive(Debug, Clone, Deserialize)]
pub struct CircleElement {
    pub id: String32,
    pub center: [u16; 2],
    pub radius: u16,
    pub fill_importance: Option<Importance>,
    pub stroke_importance: Option<Importance>,
    pub stroke_thickness: u16,
}

// ==================== 渲染引擎核心结构 ====================

/// 渲染上下文，包含绘制所需的所有资源
pub struct RenderContext<'a> {
    pub display_driver: &'a mut dyn DisplayDriver,
    pub data_registry: &'a DataSourceRegistry,
    pub fonts: &'a FontCollection,
}

/// 布局渲染引擎
pub struct RenderEngine {
    root_layout: Option<LayoutNode>,
    data_registry: &'static DataSourceRegistry,
    fonts: &'static FontCollection,
}

impl RenderEngine {
    /// 创建新的渲染引擎实例
    pub fn new(
        data_registry: &'static DataSourceRegistry,
        fonts: &'static FontCollection,
    ) -> Result<Self> {
        // 加载并解析布局数据
        let root_layout = Self::load_layout()?;

        Ok(Self {
            root_layout: Some(root_layout),
            data_registry,
            fonts,
        })
    }

    /// 从编译期生成的二进制数据加载布局
    fn load_layout() -> Result<LayoutNode> {
        let layout_data = MAIN_LAYOUT_BIN;

        // 使用postcard反序列化布局数据
        let layout: LayoutNode = from_bytes(layout_data)
            .map_err(|e| AppError::LayoutError(format!("反序列化布局失败: {}", e)))?;

        Ok(layout)
    }

    /// 渲染整个布局到屏幕
    pub async fn render(&self, context: &mut RenderContext<'_>) -> Result<()> {
        // 清屏
        context.display_driver.clear()?;

        // 获取根布局
        let root_layout = self
            .root_layout
            .as_ref()
            .ok_or_else(|| AppError::LayoutError("布局未加载".to_string()))?;

        // 渲染根布局
        self.render_node(root_layout, context, &LayoutRect::full_screen())?;

        // 刷新屏幕显示
        context.display_driver.flush()?;

        Ok(())
    }

    /// 渲染单个布局节点
    fn render_node(
        &self,
        node: &LayoutNode,
        context: &mut RenderContext<'_>,
        parent_rect: &LayoutRect,
    ) -> Result<()> {
        match node {
            LayoutNode::Container(container) => {
                self.render_container(container, context, parent_rect)
            }
            LayoutNode::Text(text) => self.render_text(text, context, parent_rect),
            LayoutNode::Icon(icon) => self.render_icon(icon, context, parent_rect),
            LayoutNode::Line(line) => self.render_line(line, context, parent_rect),
            LayoutNode::Rectangle(rect) => self.render_rectangle(rect, context, parent_rect),
            LayoutNode::Circle(circle) => self.render_circle(circle, context, parent_rect),
        }
    }

    /// 渲染容器元素
    fn render_container(
        &self,
        container: &Container,
        context: &mut RenderContext<'_>,
        parent_rect: &LayoutRect,
    ) -> Result<()> {
        // 检查条件
        if let Some(condition) = &container.condition {
            if !self.evaluate_condition(condition, context)? {
                return Ok(());
            }
        }

        // 计算容器的实际矩形（考虑父容器的偏移）
        let container_rect = LayoutRect::from_array(container.rect).relative_to(parent_rect);

        // 渲染容器边框
        self.render_container_border(container, context, &container_rect)?;

        // 计算内部可用区域（减去边框）
        let inner_rect = container_rect.inner_rect(&container.border);

        // 计算权重布局
        let child_layouts = self.calculate_child_layouts(container, &inner_rect)?;

        // 渲染子元素
        for child_layout in child_layouts {
            self.render_node(&child_layout.node, context, &child_layout.rect)?;
        }

        Ok(())
    }

    /// 渲染容器边框
    fn render_container_border(
        &self,
        container: &Container,
        context: &mut RenderContext<'_>,
        rect: &LayoutRect,
    ) -> Result<()> {
        let border = &container.border;

        // 如果有边框，绘制四条边
        if border.top > 0 {
            let top_line = GfxLine::new(
                Point::new(rect.x as i32, rect.y as i32),
                Point::new((rect.x + rect.width) as i32, rect.y as i32),
            )
            .into_styled(PrimitiveStyle::with_stroke(
                Self::importance_to_color(&Importance::Normal),
                border.top as u32,
            ));

            top_line.draw(context.display_driver)?;
        }

        if border.bottom > 0 {
            let bottom_y = rect.y + rect.height - border.top;
            let bottom_line = GfxLine::new(
                Point::new(rect.x as i32, bottom_y as i32),
                Point::new((rect.x + rect.width) as i32, bottom_y as i32),
            )
            .into_styled(PrimitiveStyle::with_stroke(
                Self::importance_to_color(&Importance::Normal),
                border.bottom as u32,
            ));

            bottom_line.draw(context.display_driver)?;
        }

        if border.left > 0 {
            let left_line = GfxLine::new(
                Point::new(rect.x as i32, rect.y as i32),
                Point::new(rect.x as i32, (rect.y + rect.height) as i32),
            )
            .into_styled(PrimitiveStyle::with_stroke(
                Self::importance_to_color(&Importance::Normal),
                border.left as u32,
            ));

            left_line.draw(context.display_driver)?;
        }

        if border.right > 0 {
            let right_x = rect.x + rect.width - border.right;
            let right_line = GfxLine::new(
                Point::new(right_x as i32, rect.y as i32),
                Point::new(right_x as i32, (rect.y + rect.height) as i32),
            )
            .into_styled(PrimitiveStyle::with_stroke(
                Self::importance_to_color(&Importance::Normal),
                border.right as u32,
            ));

            right_line.draw(context.display_driver)?;
        }

        Ok(())
    }

    /// 计算子元素布局
    fn calculate_child_layouts(
        &self,
        container: &Container,
        inner_rect: &LayoutRect,
    ) -> Result<Vec<ChildLayoutWithRect>> {
        let mut child_layouts = Vec::new();

        // 分离绝对定位和相对定位的子元素
        let (absolute_children, relative_children): (Vec<_>, Vec<_>) = container
            .children
            .iter()
            .partition(|child| child.is_absolute);

        // 处理绝对定位的子元素
        for child in absolute_children {
            let child_rect = LayoutRect::from_array(child.node.rect()).relative_to(inner_rect);

            child_layouts.push(ChildLayoutWithRect {
                node: child.node.clone(),
                rect: child_rect,
            });
        }

        // 处理相对定位的子元素（权重布局）
        if !relative_children.is_empty() {
            let calculated_rects = match container.direction {
                ContainerDirection::Horizontal => {
                    self.calculate_horizontal_layout(&relative_children, inner_rect)
                }
                ContainerDirection::Vertical => {
                    self.calculate_vertical_layout(&relative_children, inner_rect)
                }
            }?;

            for (i, rect) in calculated_rects.into_iter().enumerate() {
                child_layouts.push(ChildLayoutWithRect {
                    node: relative_children[i].node.clone(),
                    rect: rect.relative_to(inner_rect),
                });
            }
        }

        Ok(child_layouts)
    }

    /// 计算水平布局
    fn calculate_horizontal_layout(
        &self,
        children: &[&ChildLayout],
        container_rect: &LayoutRect,
    ) -> Result<Vec<LayoutRect>> {
        let total_weight: f32 = children
            .iter()
            .filter_map(|child| child.weight)
            .filter(|&w| w > 0.0)
            .sum();

        let mut result = Vec::new();
        let mut current_x = 0;

        for child in children {
            let weight = child.weight.unwrap_or(1.0);
            let child_width = if weight > 0.0 {
                // 按权重分配宽度
                ((container_rect.width as f32) * (weight / total_weight)) as u16
            } else {
                // 使用元素自身的宽度
                child.node.rect()[2]
            };

            let rect = LayoutRect {
                x: current_x,
                y: 0,
                width: child_width.min(container_rect.width - current_x),
                height: container_rect.height,
            };

            result.push(rect);
            current_x += child_width;
        }

        Ok(result)
    }

    /// 计算垂直布局
    fn calculate_vertical_layout(
        &self,
        children: &[&ChildLayout],
        container_rect: &LayoutRect,
    ) -> Result<Vec<LayoutRect>> {
        let total_weight: f32 = children
            .iter()
            .filter_map(|child| child.weight)
            .filter(|&w| w > 0.0)
            .sum();

        let mut result = Vec::new();
        let mut current_y = 0;

        for child in children {
            let weight = child.weight.unwrap_or(1.0);
            let child_height = if weight > 0.0 {
                // 按权重分配高度
                ((container_rect.height as f32) * (weight / total_weight)) as u16
            } else {
                // 使用元素自身的高度
                child.node.rect()[3]
            };

            let rect = LayoutRect {
                x: 0,
                y: current_y,
                width: container_rect.width,
                height: child_height.min(container_rect.height - current_y),
            };

            result.push(rect);
            current_y += child_height;
        }

        Ok(result)
    }

    /// 渲染文本元素
    fn render_text(
        &self,
        text: &Text,
        context: &mut RenderContext<'_>,
        parent_rect: &LayoutRect,
    ) -> Result<()> {
        // 获取实际文本内容（替换占位符）
        let actual_content = self.replace_placeholders(&text.content, context)?;

        if actual_content.is_empty() {
            return Ok(());
        }

        // 获取字体
        let font = self
            .fonts
            .get(&text.font_size)
            .ok_or_else(|| AppError::LayoutError(format!("字体未找到: {}", text.font_size)))?;

        // 计算文本位置
        let text_rect = LayoutRect::from_array(text.rect).relative_to(parent_rect);

        // 计算文本对齐的起始位置
        let start_point = self.calculate_text_position(
            &actual_content,
            font,
            &text_rect,
            text.alignment,
            text.vertical_alignment,
        );

        // 创建文本样式
        let text_style =
            MonoTextStyle::new(&font.font, Self::importance_to_color(&Importance::Normal));

        // 处理文本换行
        if let Some(max_width) = text.max_width {
            self.render_text_with_wrapping(
                &actual_content,
                start_point,
                max_width,
                text.max_lines.unwrap_or(u8::MAX),
                text_style,
                context.display_driver,
            )
        } else {
            // 单行文本
            let text_obj =
                embedded_graphics::text::Text::new(&actual_content, start_point, text_style);

            text_obj
                .draw(context.display_driver)
                .map_err(|e| AppError::RenderError(format!("文本渲染失败: {}", e)))
        }
    }

    /// 渲染带换行的文本
    fn render_text_with_wrapping(
        &self,
        content: &str,
        start_point: Point,
        max_width: u16,
        max_lines: u8,
        text_style: MonoTextStyle<BinaryColor>,
        display: &mut dyn DisplayDriver,
    ) -> Result<()> {
        let font_height = text_style.font.character_size.height;
        let mut current_point = start_point;
        let mut lines_rendered = 0;

        // 简单的换行算法（按字符分割）
        let mut current_line = String::<256>::new();

        for ch in content.chars() {
            if ch == '\n' {
                // 渲染当前行
                if !current_line.is_empty() {
                    let text_obj = embedded_graphics::text::Text::new(
                        &current_line,
                        current_point,
                        text_style,
                    );

                    text_obj
                        .draw(display)
                        .map_err(|e| AppError::RenderError(format!("文本渲染失败: {}", e)))?;

                    current_line.clear();
                    current_point.y += font_height as i32;
                    lines_rendered += 1;
                }

                if lines_rendered >= max_lines {
                    break;
                }
                continue;
            }

            // 检查当前行宽度
            current_line
                .push(ch)
                .map_err(|_| AppError::RenderError("文本内容过长".to_string()))?;

            // 这里可以添加更精确的宽度计算
            // 简化处理：如果行太长，就换行
            if current_line.len()
                > (max_width / text_style.font.character_size.width as u16) as usize
            {
                // 移除最后一个字符并换行
                current_line.pop();

                if !current_line.is_empty() {
                    let text_obj = embedded_graphics::text::Text::new(
                        &current_line,
                        current_point,
                        text_style,
                    );

                    text_obj
                        .draw(display)
                        .map_err(|e| AppError::RenderError(format!("文本渲染失败: {}", e)))?;

                    current_line.clear();
                    current_point.y += font_height as i32;
                    lines_rendered += 1;

                    if lines_rendered >= max_lines {
                        break;
                    }
                }

                // 在新行添加当前字符
                current_line
                    .push(ch)
                    .map_err(|_| AppError::RenderError("文本内容过长".to_string()))?;
            }
        }

        // 渲染最后一行
        if !current_line.is_empty() && lines_rendered < max_lines {
            let text_obj =
                embedded_graphics::text::Text::new(&current_line, current_point, text_style);

            text_obj
                .draw(display)
                .map_err(|e| AppError::RenderError(format!("文本渲染失败: {}", e)))?;
        }

        Ok(())
    }

    /// 渲染图标元素
    fn render_icon(
        &self,
        icon: &IconElement,
        context: &mut RenderContext<'_>,
        parent_rect: &LayoutRect,
    ) -> Result<()> {
        // 获取实际的图标ID（替换占位符）
        let actual_icon_id = self.replace_placeholders(&icon.icon_id, context)?;

        if actual_icon_id.is_empty() {
            return Ok(());
        }

        // 查找图标数据
        let icon_data = self.find_icon_data(&actual_icon_id)?;

        // 计算图标位置
        let icon_rect = LayoutRect::from_array(icon.rect).relative_to(parent_rect);

        // 创建图标图像
        let image = Image::new(
            &icon_data,
            Point::new(icon_rect.x as i32, icon_rect.y as i32),
        );

        // 绘制图标
        image
            .draw(context.display_driver)
            .map_err(|e| AppError::RenderError(format!("图标渲染失败: {}", e)))
    }

    /// 渲染线条元素
    fn render_line(
        &self,
        line: &LineElement,
        context: &mut RenderContext<'_>,
        parent_rect: &LayoutRect,
    ) -> Result<()> {
        // 计算实际位置（相对于父容器）
        let start_x = line.start[0] as i32 + parent_rect.x as i32;
        let start_y = line.start[1] as i32 + parent_rect.y as i32;
        let end_x = line.end[0] as i32 + parent_rect.x as i32;
        let end_y = line.end[1] as i32 + parent_rect.y as i32;

        // 获取颜色
        let color = match line.importance {
            Some(imp) => Self::importance_to_color(&imp),
            None => Self::importance_to_color(&Importance::Normal),
        };

        // 创建线条
        let line_obj = GfxLine::new(Point::new(start_x, start_y), Point::new(end_x, end_y))
            .into_styled(PrimitiveStyle::with_stroke(color, line.thickness as u32));

        // 绘制线条
        line_obj
            .draw(context.display_driver)
            .map_err(|e| AppError::RenderError(format!("线条渲染失败: {}", e)))
    }

    /// 渲染矩形元素
    fn render_rectangle(
        &self,
        rect: &RectangleElement,
        context: &mut RenderContext<'_>,
        parent_rect: &LayoutRect,
    ) -> Result<()> {
        // 计算实际位置
        let rect_rect = LayoutRect::from_array(rect.rect).relative_to(parent_rect);

        // 创建矩形
        let rectangle = GfxRectangle::new(
            Point::new(rect_rect.x as i32, rect_rect.y as i32),
            Size::new(rect_rect.width as u32, rect_rect.height as u32),
        );

        // 设置样式
        let mut style = PrimitiveStyle::default();

        // 填充颜色
        if let Some(fill_importance) = &rect.fill_importance {
            style = style.with_fill(Self::importance_to_color(fill_importance));
        }

        // 边框
        if rect.stroke_thickness > 0 {
            let stroke_color = match &rect.stroke_importance {
                Some(imp) => Self::importance_to_color(imp),
                None => Self::importance_to_color(&Importance::Normal),
            };

            style = style.with_stroke(stroke_color, rect.stroke_thickness as u32);
        }

        // 绘制矩形
        rectangle
            .into_styled(style)
            .draw(context.display_driver)
            .map_err(|e| AppError::RenderError(format!("矩形渲染失败: {}", e)))
    }

    /// 渲染圆形元素
    fn render_circle(
        &self,
        circle: &CircleElement,
        context: &mut RenderContext<'_>,
        parent_rect: &LayoutRect,
    ) -> Result<()> {
        // 计算实际位置
        let center_x = circle.center[0] as i32 + parent_rect.x as i32;
        let center_y = circle.center[1] as i32 + parent_rect.y as i32;

        // 创建圆形
        let circle_obj = Circle::new(
            Point::new(center_x, center_y),
            circle.radius as u32 * 2, // diameter
        );

        // 设置样式
        let mut style = PrimitiveStyle::default();

        // 填充颜色
        if let Some(fill_importance) = &circle.fill_importance {
            style = style.with_fill(Self::importance_to_color(fill_importance));
        }

        // 边框
        if circle.stroke_thickness > 0 {
            let stroke_color = match &circle.stroke_importance {
                Some(imp) => Self::importance_to_color(imp),
                None => Self::importance_to_color(&Importance::Normal),
            };

            style = style.with_stroke(stroke_color, circle.stroke_thickness as u32);
        }

        // 绘制圆形
        circle_obj
            .into_styled(style)
            .draw(context.display_driver)
            .map_err(|e| AppError::RenderError(format!("圆形渲染失败: {}", e)))
    }

    /// 替换占位符
    fn replace_placeholders(&self, text: &str, context: &RenderContext<'_>) -> Result<String> {
        if !text.contains("{{") || !text.contains("}}") {
            return Ok(text.to_string());
        }

        let mut result = String::<1024>::new();
        let mut chars = text.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '{' && chars.peek() == Some(&'{') {
                chars.next(); // 跳过第二个 '{'
                let mut placeholder = String::<128>::new();

                // 提取占位符内容
                while let Some(c) = chars.next() {
                    if c == '}' && chars.peek() == Some(&'}') {
                        chars.next(); // 跳过第二个 '}'
                        break;
                    }
                    placeholder
                        .push(c)
                        .map_err(|_| AppError::LayoutError("占位符过长".to_string()))?;
                }

                // 获取占位符对应的值
                let value = self.get_placeholder_value(&placeholder, context)?;
                result
                    .push_str(&value)
                    .map_err(|_| AppError::LayoutError("替换后文本过长".to_string()))?;
            } else {
                result
                    .push(c)
                    .map_err(|_| AppError::LayoutError("文本过长".to_string()))?;
            }
        }

        Ok(result.to_string())
    }

    /// 获取占位符对应的值
    fn get_placeholder_value(
        &self,
        placeholder: &str,
        context: &RenderContext<'_>,
    ) -> Result<String> {
        let placeholder = placeholder.trim();

        // 特殊处理数字图标格式
        if placeholder.starts_with("digital_icon") {
            return self.resolve_digital_icon(placeholder, context);
        }

        if placeholder.starts_with("weather_icon") {
            return self.resolve_weather_icon(placeholder, context);
        }

        // 普通数据占位符
        let data_source = context
            .data_registry
            .get(placeholder)
            .ok_or_else(|| AppError::LayoutError(format!("数据源未找到: {}", placeholder)))?;

        let value = data_source
            .get_value(context.system_state)
            .map_err(|e| AppError::LayoutError(format!("获取数据失败: {}", e)))?;

        // 转换为字符串
        self.value_to_string(&value)
    }

    /// 解析数字图标占位符
    fn resolve_digital_icon(
        &self,
        placeholder: &str,
        context: &RenderContext<'_>,
    ) -> Result<String> {
        // 处理固定图标，如 digital_icon::colon
        if placeholder == "digital_icon::colon" {
            return Ok("colon".to_string());
        }

        // 处理动态数字图标，如 digital_icon(time.hour_tens)
        if placeholder.starts_with("digital_icon(") && placeholder.ends_with(")") {
            let inner = &placeholder[13..placeholder.len() - 1]; // 去掉 digital_icon( 和 )

            // 获取数字值
            let data_source = context
                .data_registry
                .get(inner)
                .ok_or_else(|| AppError::LayoutError(format!("数字数据源未找到: {}", inner)))?;

            let value = data_source
                .get_value(context.system_state)
                .map_err(|e| AppError::LayoutError(format!("获取数字失败: {}", e)))?;

            // 转换为数字，然后映射为图标ID
            let num = match value {
                DynamicValue::Int(n) => n as u8,
                DynamicValue::UInt(n) => n as u8,
                DynamicValue::Float(f) => f as u8,
                _ => return Err(AppError::LayoutError(format!("不是数字类型: {}", inner))),
            };

            // 数字0-9映射为对应的图标ID
            let icon_id = match num {
                0 => "digit_0",
                1 => "digit_1",
                2 => "digit_2",
                3 => "digit_3",
                4 => "digit_4",
                5 => "digit_5",
                6 => "digit_6",
                7 => "digit_7",
                8 => "digit_8",
                9 => "digit_9",
                _ => return Err(AppError::LayoutError(format!("无效的数字: {}", num))),
            };

            return Ok(icon_id.to_string());
        }

        Err(AppError::LayoutError(format!(
            "无效的数字图标格式: {}",
            placeholder
        )))
    }

    /// 解析天气图标占位符
    fn resolve_weather_icon(
        &self,
        placeholder: &str,
        context: &RenderContext<'_>,
    ) -> Result<String> {
        // 格式: weather_icon(daily_weather[0].weather_icon)
        if placeholder.starts_with("weather_icon(") && placeholder.ends_with(")") {
            let inner = &placeholder[12..placeholder.len() - 1];

            // 这里简化处理，实际应该根据天气类型映射图标
            // 获取天气类型值
            let data_source = context
                .data_registry
                .get(inner)
                .ok_or_else(|| AppError::LayoutError(format!("天气数据源未找到: {}", inner)))?;

            let value = data_source
                .get_value(context.system_state)
                .map_err(|e| AppError::LayoutError(format!("获取天气失败: {}", e)))?;

            // 根据天气类型返回对应的图标ID
            let weather_type = self.value_to_string(&value)?;
            let icon_id = match weather_type.as_str() {
                "sunny" | "clear" => "weather_sunny",
                "cloudy" => "weather_cloudy",
                "rain" | "rainy" => "weather_rain",
                "snow" => "weather_snow",
                "fog" | "foggy" => "weather_fog",
                "thunderstorm" => "weather_thunder",
                _ => "weather_unknown",
            };

            return Ok(icon_id.to_string());
        }

        Err(AppError::LayoutError(format!(
            "无效的天气图标格式: {}",
            placeholder
        )))
    }

    /// 将值转换为字符串
    fn value_to_string(&self, value: &DynamicValue) -> Result<String> {
        match value {
            DynamicValue::String(s) => Ok(s.clone()),
            DynamicValue::Int(n) => Ok(n.to_string()),
            DynamicValue::UInt(n) => Ok(n.to_string()),
            DynamicValue::Float(f) => Ok(f.to_string()),
            DynamicValue::Bool(b) => Ok(b.to_string()),
            DynamicValue::Null => Ok("".to_string()),
            _ => Err(AppError::LayoutError("不支持的值类型".to_string())),
        }
    }

    /// 查找图标数据
    fn find_icon_data(
        &self,
        icon_id: &str,
    ) -> Result<embedded_graphics::image::ImageRaw<'static, BinaryColor>> {
        // 这里应该根据项目实际的图标数据结构来实现
        // 简化实现：假设有全局的ICON_DATA和ICON_IDS
        let index = ICON_IDS
            .iter()
            .position(|&id| id == icon_id)
            .ok_or_else(|| AppError::LayoutError(format!("图标未找到: {}", icon_id)))?;

        // 这里需要根据实际的数据结构获取图标数据
        // 简化处理：返回一个占位图标
        Ok(embedded_graphics::image::ImageRaw::new(&[0u8; 1], 1))
    }

    /// 评估条件表达式
    fn evaluate_condition(&self, condition: &str, context: &RenderContext<'_>) -> Result<bool> {
        // 简化实现：只处理简单的相等比较
        if condition.contains("==") {
            let parts: Vec<&str> = condition.split("==").map(|s| s.trim()).collect();
            if parts.len() == 2 {
                let left_value = self.get_placeholder_value(parts[0], context)?;
                let right_value = parts[1].trim_matches('"').trim_matches('\'');
                return Ok(left_value == right_value);
            }
        }

        // 处理布尔表达式
        if condition == "true" {
            return Ok(true);
        }
        if condition == "false" {
            return Ok(false);
        }

        // 处理存在性检查
        if condition.ends_with("!= ''") {
            let placeholder = condition[..condition.len() - 5].trim();
            let value = self.get_placeholder_value(placeholder, context)?;
            return Ok(!value.is_empty());
        }

        // 默认返回true，让元素显示
        Ok(true)
    }

    /// 计算文本位置（考虑对齐方式）
    fn calculate_text_position(
        &self,
        text: &str,
        font: &Font,
        rect: &LayoutRect,
        alignment: TextAlignment,
        vertical_alignment: VerticalAlignment,
    ) -> Point {
        let char_width = font.font.character_size.width as i32;
        let char_height = font.font.character_size.height as i32;

        // 计算文本宽度（简化：假设所有字符等宽）
        let text_width = text.len() as i32 * char_width;

        // 水平对齐
        let x = match alignment {
            TextAlignment::Left => rect.x as i32,
            TextAlignment::Center => rect.x as i32 + (rect.width as i32 - text_width) / 2,
            TextAlignment::Right => rect.x as i32 + rect.width as i32 - text_width,
        };

        // 垂直对齐
        let y = match vertical_alignment {
            VerticalAlignment::Top => rect.y as i32,
            VerticalAlignment::Center => rect.y as i32 + (rect.height as i32 - char_height) / 2,
            VerticalAlignment::Bottom => rect.y as i32 + rect.height as i32 - char_height,
        };

        Point::new(x, y)
    }

    /// 将重要程度转换为颜色
    fn importance_to_color(importance: &Importance) -> BinaryColor {
        match importance {
            Importance::Normal => BinaryColor::Off,  // 黑色/关闭
            Importance::Warning => BinaryColor::On,  // 白色/开启（黄色在4色屏上可能映射为白色）
            Importance::Critical => BinaryColor::On, // 白色/开启（红色在4色屏上可能映射为白色）
        }
    }
}

// ==================== 辅助结构 ====================

/// 布局矩形
#[derive(Debug, Clone, Copy)]
struct LayoutRect {
    x: u16,
    y: u16,
    width: u16,
    height: u16,
}

impl LayoutRect {
    /// 创建全屏矩形
    fn full_screen() -> Self {
        Self {
            x: 0,
            y: 0,
            width: 800,
            height: 480,
        }
    }

    /// 从数组创建
    fn from_array(rect: [u16; 4]) -> Self {
        Self {
            x: rect[0],
            y: rect[1],
            width: rect[2],
            height: rect[3],
        }
    }

    /// 转换为数组
    fn to_array(&self) -> [u16; 4] {
        [self.x, self.y, self.width, self.height]
    }

    /// 相对于父容器的位置
    fn relative_to(&self, parent: &LayoutRect) -> Self {
        Self {
            x: parent.x + self.x,
            y: parent.y + self.y,
            width: self.width.min(parent.width.saturating_sub(self.x)),
            height: self.height.min(parent.height.saturating_sub(self.y)),
        }
    }

    /// 内部矩形（减去边框）
    fn inner_rect(&self, border: &Border) -> Self {
        Self {
            x: self.x + border.left,
            y: self.y + border.top,
            width: self.width.saturating_sub(border.left + border.right),
            height: self.height.saturating_sub(border.top + border.bottom),
        }
    }
}

/// 带矩形的子布局
struct ChildLayoutWithRect {
    node: LayoutNode,
    rect: LayoutRect,
}

// 为布局元素添加rect()方法的实现
trait HasRect {
    fn rect(&self) -> [u16; 4];
}

impl HasRect for Container {
    fn rect(&self) -> [u16; 4] {
        self.rect
    }
}

impl HasRect for Text {
    fn rect(&self) -> [u16; 4] {
        self.rect
    }
}

impl HasRect for IconElement {
    fn rect(&self) -> [u16; 4] {
        self.rect
    }
}

impl HasRect for LineElement {
    fn rect(&self) -> [u16; 4] {
        // 线条需要转换为边界矩形
        let x1 = self.start[0];
        let y1 = self.start[1];
        let x2 = self.end[0];
        let y2 = self.end[1];

        let x_min = x1.min(x2);
        let y_min = y1.min(y2);
        let x_max = x1.max(x2);
        let y_max = y1.max(y2);

        let width = if x_max > x_min { x_max - x_min } else { 1 };
        let height = if y_max > y_min { y_max - y_min } else { 1 };

        [x_min, y_min, width, height]
    }
}

impl HasRect for RectangleElement {
    fn rect(&self) -> [u16; 4] {
        self.rect
    }
}

impl HasRect for CircleElement {
    fn rect(&self) -> [u16; 4] {
        let diameter = self.radius * 2;
        [
            self.center[0].saturating_sub(self.radius),
            self.center[1].saturating_sub(self.radius),
            diameter,
            diameter,
        ]
    }
}

impl HasRect for LayoutNode {
    fn rect(&self) -> [u16; 4] {
        match self {
            LayoutNode::Container(c) => c.rect(),
            LayoutNode::Text(t) => t.rect(),
            LayoutNode::Icon(i) => i.rect(),
            LayoutNode::Line(l) => l.rect(),
            LayoutNode::Rectangle(r) => r.rect(),
            LayoutNode::Circle(c) => c.rect(),
        }
    }
}

impl HasRect for ChildLayout {
    fn rect(&self) -> [u16; 4] {
        self.node.rect()
    }
}