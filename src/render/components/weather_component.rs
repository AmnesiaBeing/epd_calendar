use alloc::string::ToString;
use embedded_graphics::{
    Drawable,
    geometry::Size,
    prelude::{DrawTarget, Point},
    primitives::Rectangle,
};
use epd_waveshare::color::QuadColor;

use crate::{
    assets::{generated_fonts::FontSize, generated_weather_icons::get_weather_icon_data},
    common::{system_state::WeatherData, weather::DailyWeather},
    render::{TextRenderer, draw_binary_image, text_renderer::TextAlignment},
};

// 天气显示总容器
const WEATHER_RECT: Rectangle = Rectangle::new(Point::new(245, 135), Size::new(555, 265));
// 排版间距配置
const ICON_TEXT_SPACING: i32 = 8; // 图标与文字的间距
const LINE_SPACING: u32 = 12; // 行间距
const SENSOR_ITEM_SPACING: i32 = 60; // 传感器数据项间距
const DAY_WEATHER_PADDING: i32 = 5; // 单日天气区域内边距
const ICON_SIZE: Size = Size::new(32, 32); // 天气图标尺寸

// 格式化温度显示（保留一位小数，处理NaN）
fn format_temperature(temp: f32) -> alloc::string::String {
    if temp.is_nan() || temp == f32::INFINITY || temp == f32::NEG_INFINITY {
        "--".to_string()
    } else {
        alloc::format!("{:.1}", temp)
    }
}

// 格式化湿度显示（整数，处理NaN）
fn format_humidity(humidity: f32) -> alloc::string::String {
    if humidity.is_nan() || humidity == f32::INFINITY || humidity == f32::NEG_INFINITY {
        "--".to_string()
    } else {
        alloc::format!("{:.0}", humidity)
    }
}

// 获取日期标签（今天/明天/后天）
fn get_day_label(index: usize) -> &'static str {
    match index {
        0 => "今天",
        1 => "明天",
        2 => "后天",
        _ => "",
    }
}

// 辅助函数：绘制温湿度传感器数据（横向大字体居中布局）
fn draw_sensor_data<D>(
    target: &mut D,
    temp: f32,
    humidity: f32,
    rect: Rectangle,
) -> Result<(), D::Error>
where
    D: DrawTarget<Color = QuadColor>,
{
    // 格式化温湿度文本
    let temp_text = alloc::format!("温度: {}℃", format_temperature(temp));
    let humidity_text = alloc::format!("湿度: {}%", format_humidity(humidity));

    // 创建Large字体渲染器
    let mut renderer = TextRenderer::new(FontSize::Large, Point::zero());

    // 计算文本尺寸
    let temp_width = renderer.calculate_text_width(&temp_text);
    let humidity_width = renderer.calculate_text_width(&humidity_text);
    let total_content_width = temp_width + humidity_width + SENSOR_ITEM_SPACING;

    // 计算整体居中位置（水平+垂直）
    let center_x = rect.top_left.x + (rect.size.width as i32 - total_content_width) / 2;
    let center_y =
        rect.top_left.y + (rect.size.height as i32 - renderer.get_line_height() as i32) / 2;

    // 绘制温度文本
    renderer.move_to(Point::new(center_x, center_y));
    renderer.draw_single_line(target, &temp_text)?;

    // 绘制湿度文本（温度右侧+间距）
    renderer.move_to(Point::new(
        center_x + temp_width + SENSOR_ITEM_SPACING,
        center_y,
    ));
    renderer.draw_single_line(target, &humidity_text)?;

    Ok(())
}

// 辅助函数：绘制单天天气信息（图标+文字）
fn draw_single_day_weather<D>(
    target: &mut D,
    day: &DailyWeather,
    day_label: &str,
    font_size: FontSize,
    rect: Rectangle,
) -> Result<(), D::Error>
where
    D: DrawTarget<Color = QuadColor>,
{
    // 创建文本渲染器
    let mut renderer = TextRenderer::new(font_size, Point::zero());

    // 构建天气信息文本
    let weather_info = alloc::format!(
        "{}: {} {:.0}°~{:.0}° 湿度:{}%",
        day_label,
        day.text_day,
        day.temp_max,
        day.temp_min,
        day.humidity
    );

    // 计算内容起始位置（区域内边距）
    let content_x = rect.top_left.x + DAY_WEATHER_PADDING;
    let content_y =
        rect.top_left.y + (rect.size.height as i32 - renderer.get_line_height() as i32) / 2;

    // 绘制天气图标（如果有）
    let mut text_x = content_x;
    if let Some(icon) =
        crate::assets::generated_weather_icons::get_weather_icon_from_code(&day.icon_day)
    {
        let icon_data = get_weather_icon_data(icon);
        // 图标垂直居中：图标Y坐标 = 内容Y - (图标高度 - 字体行高)/2
        let icon_y =
            content_y - ((ICON_SIZE.height as i32) - renderer.get_line_height() as i32) / 2;
        draw_binary_image(target, icon_data, ICON_SIZE, Point::new(content_x, icon_y))?;

        // 文字在图标右侧
        text_x += ICON_SIZE.width as i32 + ICON_TEXT_SPACING;
    }

    // 绘制天气文字（自动换行适配区域宽度）
    let text_max_width = rect.size.width as i32 - (text_x - rect.top_left.x) - DAY_WEATHER_PADDING;
    renderer.move_to(Point::new(text_x, content_y));
    renderer.draw_text_multiline(target, &weather_info, text_max_width, TextAlignment::Left)?;

    Ok(())
}

// 辅助函数：绘制天气预报信息（位置+三天预报）
fn draw_weather_forecast<D>(
    target: &mut D,
    location: &str,
    daily_forecast: &[DailyWeather; 3],
    sensor_data: Option<(f32, f32)>,
    rect: Rectangle,
) -> Result<(), D::Error>
where
    D: DrawTarget<Color = QuadColor>,
{
    // ========== 1. 绘制第一行：位置+传感器数据（如果有） ==========
    let mut location_renderer = TextRenderer::new(FontSize::Medium, Point::zero());

    // 构建第一行文本
    let first_line = if let Some((temp, humidity)) = sensor_data {
        alloc::format!(
            "{} 温度:{}℃ 湿度:{}%",
            location,
            format_temperature(temp),
            format_humidity(humidity)
        )
    } else {
        location.to_string()
    };

    // 计算第一行居中位置
    let first_line_width = location_renderer.calculate_text_width(&first_line);
    let first_line_x = rect.top_left.x + (rect.size.width as i32 - first_line_width) / 2;
    let first_line_y = rect.top_left.y;

    // 绘制第一行
    location_renderer.move_to(Point::new(first_line_x, first_line_y));
    location_renderer.draw_single_line(target, &first_line)?;

    // ========== 2. 绘制三天天气预报 ==========
    // 计算单日天气区域尺寸（平均分配宽度，减去间距）
    let day_area_total_width = rect.size.width as i32;
    let day_area_count = 3;
    let day_area_spacing = 5; // 单日区域间的间距
    let day_area_width =
        (day_area_total_width - (day_area_count - 1) as i32 * day_area_spacing) / day_area_count;
    let day_area_height =
        rect.size.height as i32 - location_renderer.get_line_height() as i32 - LINE_SPACING as i32;

    // 计算起始Y坐标
    let day_area_start_y =
        first_line_y + location_renderer.get_line_height() as i32 + LINE_SPACING as i32;

    // 绘制每一天的天气
    for (i, day) in daily_forecast.iter().enumerate() {
        // 计算单日区域位置
        let day_area_x = rect.top_left.x + (i as i32) * (day_area_width + day_area_spacing);
        let day_area_rect = Rectangle::new(
            Point::new(day_area_x, day_area_start_y),
            Size::new(day_area_width as u32, day_area_height as u32),
        );

        // 选择字体尺寸：今天用Medium，明后天用Small
        let font_size = if i == 0 {
            FontSize::Medium
        } else {
            FontSize::Small
        };

        // 获取日期标签
        let day_label = get_day_label(i);

        // 绘制单日天气
        draw_single_day_weather(target, day, day_label, font_size, day_area_rect)?;
    }

    Ok(())
}

impl Drawable for &WeatherData {
    type Color = QuadColor;
    type Output = ();

    fn draw<D>(&self, target: &mut D) -> Result<Self::Output, D::Error>
    where
        D: DrawTarget<Color = Self::Color>,
    {
        // 数据优先级：天气预报 > 传感器数据 > 无数据提示
        if let Some((location, daily_forecast)) = &self.daily_forecast {
            // 有天气预报数据
            draw_weather_forecast(
                target,
                location,
                daily_forecast,
                self.sensor_data,
                WEATHER_RECT,
            )?;
        } else if let Some((temp, humidity)) = self.sensor_data {
            // 只有传感器数据
            draw_sensor_data(target, temp, humidity, WEATHER_RECT)?;
        } else {
            // 无任何天气数据，显示提示
            let mut renderer = TextRenderer::new(FontSize::Medium, Point::zero());
            let hint_text = "无天气数据";

            // 计算提示文本居中位置
            let text_width = renderer.calculate_text_width(hint_text);
            let text_height = renderer.get_line_height() as i32;
            let text_x =
                WEATHER_RECT.top_left.x + (WEATHER_RECT.size.width as i32 - text_width) / 2;
            let text_y =
                WEATHER_RECT.top_left.y + (WEATHER_RECT.size.height as i32 - text_height) / 2;

            renderer.move_to(Point::new(text_x, text_y));
            renderer.draw_single_line(target, hint_text)?;
        }

        Ok(())
    }
}
