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
    render::{TextRenderer, draw_binary_image},
};

const WEATHER_RECT: Rectangle = Rectangle::new(Point::new(245, 135), Size::new(555, 265));

// 格式化温度显示（保留一位小数）
fn format_temperature(temp: f32) -> alloc::string::String {
    if temp.is_nan() {
        "--".to_string()
    } else {
        alloc::format!("{:.1}", temp)
    }
}

// 格式化湿度显示（整数）
fn format_humidity(humidity: f32) -> alloc::string::String {
    if humidity.is_nan() {
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

// 辅助函数：绘制温湿度传感器数据（横向布局）
fn draw_sensor_data<D>(
    target: &mut D,
    temp: f32,
    humidity: f32,
    rect: Rectangle,
) -> Result<(), D::Error>
where
    D: DrawTarget<Color = QuadColor>,
{
    // 格式化温度字符串，保留一位小数
    let temp_text = alloc::format!("温度: {}℃", format_temperature(temp));

    // 格式化湿度字符串，整数
    let humidity_text = alloc::format!("湿度: {}%", format_humidity(humidity));

    // 创建文本渲染器（使用Large字体）
    let mut renderer = TextRenderer::new(
        FontSize::Large,
        Point::new(rect.top_left.x, rect.top_left.y),
    );

    // 计算两个文本的总宽度
    let temp_width = renderer.calculate_text_width(&temp_text);
    let humidity_width = renderer.calculate_text_width(&humidity_text);
    let total_width = temp_width + humidity_width;

    // 计算间距（在文本之间添加一些空白）
    let spacing = 60; // 像素间距

    // 计算起始位置使整体居中
    let start_x = rect.top_left.x + (rect.size.width as i32 - total_width as i32 - spacing) / 2;

    // 计算垂直居中位置
    let text_height = renderer.full_font_metrics.1 as i32;
    let start_y = rect.top_left.y + (rect.size.height as i32 - text_height) / 2;

    // 绘制温度
    renderer.move_to(Point::new(start_x, start_y));
    renderer.draw_text(target, &temp_text)?;

    // 绘制湿度（在温度右侧，加上间距）
    renderer.move_to(Point::new(start_x + temp_width as i32 + spacing, start_y));
    renderer.draw_text(target, &humidity_text)?;

    Ok(())
}

// 辅助函数：绘制单天天气信息
fn draw_single_day_weather<D>(
    target: &mut D,
    day: &DailyWeather,
    day_label: &str,
    font_size: FontSize,
    position: Point,
) -> Result<(), D::Error>
where
    D: DrawTarget<Color = QuadColor>,
{
    // 创建文本渲染器
    let mut renderer = TextRenderer::new(font_size, position);

    // 获取图标尺寸
    let icon_size = Size::new(32, 32);

    // 构建天气信息字符串
    let weather_info = alloc::format!(
        "{}:{} {:.0}°~{:.0}° 湿度:{}%",
        day_label,
        day.text_day,
        day.temp_max,
        day.temp_min,
        day.humidity
    );

    // 如果有天气图标，先绘制图标
    if let Some(icon) =
        crate::assets::generated_weather_icons::get_weather_icon_from_code(&day.icon_day)
    {
        let icon_data = get_weather_icon_data(icon);
        // 绘制图标
        draw_binary_image(target, icon_data, icon_size, position)?;
    }

    // 计算文字起始位置（图标右侧）
    let text_x = position.x + icon_size.width as i32 + 5;

    // 调整渲染器位置绘制文字
    renderer.move_to(Point::new(text_x, position.y));
    renderer.draw_text(target, &weather_info)?;

    Ok(())
}

// 辅助函数：绘制天气预报信息
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
    // 第一行：位置信息（使用Medium字体）
    let mut location_renderer = TextRenderer::new(
        FontSize::Medium,
        Point::new(rect.top_left.x, rect.top_left.y),
    );

    // 如果有传感器数据，构建完整的第一行
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

    // 绘制第一行
    location_renderer.draw_text(target, &first_line)?;

    // 计算每天天气信息的起始位置
    let line_spacing = 10; // 行间距
    let day_height = location_renderer.full_font_metrics.1 as i32 + line_spacing;
    let start_y = rect.top_left.y + day_height;

    // 计算每天可用的宽度（平均分配）
    let day_width = rect.size.width as i32 / 3;

    // 绘制三天的天气信息
    for (i, day) in daily_forecast.iter().enumerate() {
        let day_x = rect.top_left.x + (i as i32) * day_width;
        let day_position = Point::new(day_x, start_y);

        // 根据索引选择字体大小：今天用Medium，明后天用Small
        let font_size = if i == 0 {
            FontSize::Medium
        } else {
            FontSize::Small
        };

        // 获取日期标签
        let day_label = get_day_label(i);

        // 绘制单天天气
        draw_single_day_weather(target, day, day_label, font_size, day_position)?;
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
        // 判断数据优先级
        if let Some((location, daily_forecast)) = &self.daily_forecast {
            // 有天气预报数据，使用方案B绘制
            draw_weather_forecast(
                target,
                location,
                daily_forecast,
                self.sensor_data,
                WEATHER_RECT,
            )?;
        } else if let Some((temp, humidity)) = self.sensor_data {
            // 只有传感器数据，使用横向大字体布局
            draw_sensor_data(target, temp, humidity, WEATHER_RECT)?;
        } else {
            // 没有数据，显示提示信息
            let mut renderer = TextRenderer::new(FontSize::Medium, WEATHER_RECT.top_left);

            // 计算居中位置
            let text = "无天气数据";
            let text_width = renderer.calculate_text_width(text) as i32;
            let text_height = renderer.full_font_metrics.1 as i32;
            let text_x =
                WEATHER_RECT.top_left.x + (WEATHER_RECT.size.width as i32 - text_width) / 2;
            let text_y =
                WEATHER_RECT.top_left.y + (WEATHER_RECT.size.height as i32 - text_height) / 2;

            renderer.move_to(Point::new(text_x, text_y));
            renderer.draw_text(target, text)?;
        }

        Ok(())
    }
}
