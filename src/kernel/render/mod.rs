//! 渲染引擎模块
//! 整合文本/图标渲染，调用布局规则完成800x480信息面板的整体渲染

pub mod icon_renderer;
pub mod text_renderer;

use core::fmt::Debug;
use core::str::FromStr;

use alloc::{
    format,
    string::{String, ToString},
};
use embedded_graphics::draw_target::DrawTarget;
use embedded_graphics::geometry::OriginDimensions;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{Line, PrimitiveStyle};
use epd_waveshare::color::QuadColor;

use self::icon_renderer::IconRenderer;
use self::text_renderer::{TextAlignment, TextRenderer, VerticalAlignment};
use crate::assets::generated_fonts::FontSize;
use crate::common::error::{AppError, Result};
use crate::kernel::data::DynamicValue;
use crate::kernel::data::scheduler::DataSourceRegistry;
use crate::kernel::data::types::CacheKeyValueMap;

/// 全局渲染引擎实例
pub static DEFAULT_ENGINE: RenderEngine = RenderEngine;

/// 渲染引擎
pub struct RenderEngine;

impl RenderEngine {
    /// 获取缓存中的字符串值
    fn get_cache_string(&self, cache: &CacheKeyValueMap, key: &str) -> Result<String> {
        log::debug!("尝试获取缓存字符串值: {}", key);
        cache
            .get(key)
            .ok_or_else(|| {
                log::error!("缓存键不存在: {}", key);
                AppError::ConvertError
            })
            .and_then(|value| match value {
                DynamicValue::String(s) => {
                    log::debug!("缓存键 {} 为字符串类型: {}", key, s);
                    Ok(s.to_string())
                },
                DynamicValue::Integer(i) => {
                    log::debug!("缓存键 {} 为整数类型，转换为字符串: {}", key, i);
                    Ok(i.to_string())
                },
                DynamicValue::Float(f) => {
                    log::debug!("缓存键 {} 为浮点数类型，转换为字符串: {}", key, f);
                    Ok(f.to_string())
                },
                DynamicValue::Boolean(b) => {
                    log::debug!("缓存键 {} 为布尔类型，转换为字符串: {}", key, b);
                    Ok(b.to_string())
                },
            })
    }

    /// 获取缓存中的整数值
    fn get_cache_integer(&self, cache: &CacheKeyValueMap, key: &str) -> Result<i32> {
        log::debug!("尝试获取缓存整数值: {}", key);
        cache
            .get(key)
            .ok_or_else(|| {
                log::error!("缓存键不存在: {}", key);
                AppError::ConvertError
            })
            .and_then(|value| match value {
                DynamicValue::Integer(i) => {
                    log::debug!("缓存键 {} 为整数类型: {}", key, i);
                    Ok(*i)
                },
                DynamicValue::String(s) => {
                    log::debug!("缓存键 {} 为字符串类型，尝试转换为整数: {}", key, s);
                    i32::from_str(s.as_str()).map_err(|_| {
                        log::error!("字符串转整数失败: {}", s);
                        AppError::ConvertError
                    })
                },
                DynamicValue::Float(f) => {
                    log::debug!("缓存键 {} 为浮点数类型，转换为整数: {} -> {}", key, f, *f as i32);
                    Ok(*f as i32)
                },
                DynamicValue::Boolean(b) => {
                    let result = if *b { 1 } else { 0 };
                    log::debug!("缓存键 {} 为布尔类型，转换为整数: {} -> {}", key, b, result);
                    Ok(result)
                },
            })
    }

    /// 渲染整个布局到显示缓冲区
    pub fn render_layout<DT>(
        &self,
        target: &mut DT,
        _data_source_registry: &DataSourceRegistry,
        cache: &CacheKeyValueMap,
    ) -> Result<bool>
    where
        DT: DrawTarget<Color = QuadColor> + OriginDimensions,
        DT::Error: Debug,
    {
        log::info!("开始渲染布局");

        // 获取屏幕尺寸
        let display_size = target.size();
        let width = display_size.width as u16;
        let _height = display_size.height as u16;

        // 创建渲染器实例
        let text_renderer = TextRenderer::new();
        let icon_renderer = IconRenderer::new();
        log::debug!("渲染器实例创建完成");

        // 绘制时间（使用图标渲染器）
        log::debug!("开始绘制时间");
        self.render_time(target, &icon_renderer, cache, width)?;
        log::debug!("时间绘制完成");

        // 绘制日期
        log::debug!("开始绘制日期");
        self.render_date(target, &text_renderer, cache, width)?;
        log::debug!("日期绘制完成");

        // 绘制分割线1
        log::debug!("开始绘制分割线1");
        self.render_divider1(target, width)?;
        log::debug!("分割线1绘制完成");

        // 绘制农历和天气
        log::debug!("开始绘制农历和天气");
        self.render_lunar_and_weather(target, &text_renderer, &icon_renderer, cache, width)?;
        log::debug!("农历和天气绘制完成");

        // 绘制分割线2
        log::debug!("开始绘制分割线2");
        self.render_divider2(target, width)?;
        log::debug!("分割线2绘制完成");

        // 绘制格言
        log::debug!("开始绘制格言");
        self.render_motto(target, &text_renderer, cache, width)?;
        log::debug!("格言绘制完成");

        log::info!("布局渲染完成");
        Ok(true)
    }

    /// 绘制时间（使用图标渲染器）
    fn render_time<DT>(
        &self,
        target: &mut DT,
        icon_renderer: &IconRenderer,
        cache: &CacheKeyValueMap,
        width: u16,
    ) -> Result<()>
    where
        DT: DrawTarget<Color = QuadColor>,
        DT::Error: Debug,
    {
        log::info!("开始绘制时间，屏幕宽度: {}", width);
        // 获取时间数据
        let hour_tens = self.get_cache_integer(cache, "datetime.hour_tens")?;
        log::debug!("获取到小时十位: {}", hour_tens);
        let hour_ones = self.get_cache_integer(cache, "datetime.hour_ones")?;
        log::debug!("获取到小时个位: {}", hour_ones);
        let minute_tens = self.get_cache_integer(cache, "datetime.minute_tens")?;
        log::debug!("获取到分钟十位: {}", minute_tens);
        let minute_ones = self.get_cache_integer(cache, "datetime.minute_ones")?;
        log::debug!("获取到分钟个位: {}", minute_ones);

        // 计算时间图标的总宽度
        let icon_width = 40; // 假设每个数字图标宽度为40px
        let colon_width = 20; // 假设冒号图标宽度为20px
        let total_width = icon_width * 4 + colon_width * 2;
        log::debug!("时间图标总宽度: {}", total_width);

        // 计算起始X坐标（水平居中）
        let start_x = (width - total_width) / 2;
        let y = 20; // 顶部边距
        log::debug!("时间图标起始位置: x={}, y={}", start_x, y);

        // 绘制时间图标
        log::debug!("开始绘制小时十位图标");
        icon_renderer.render(
            target,
            [start_x, y, icon_width, icon_width],
            &format!("digit_{}", hour_tens),
        )?;
        log::debug!("开始绘制小时个位图标");
        icon_renderer.render(
            target,
            [start_x + icon_width, y, icon_width, icon_width],
            &format!("digit_{}", hour_ones),
        )?;
        log::debug!("开始绘制冒号图标");
        icon_renderer.render(
            target,
            [start_x + icon_width * 2, y, colon_width, icon_width],
            "digit_colon",
        )?;
        log::debug!("开始绘制分钟十位图标");
        icon_renderer.render(
            target,
            [
                start_x + icon_width * 2 + colon_width,
                y,
                icon_width,
                icon_width,
            ],
            &format!("digit_{}", minute_tens),
        )?;
        log::debug!("开始绘制分钟个位图标");
        icon_renderer.render(
            target,
            [
                start_x + icon_width * 3 + colon_width,
                y,
                icon_width,
                icon_width,
            ],
            &format!("digit_{}", minute_ones),
        )?;
        log::info!("时间绘制完成");

        Ok(())
    }

    /// 绘制日期
    fn render_date<DT>(
        &self,
        target: &mut DT,
        text_renderer: &TextRenderer,
        cache: &CacheKeyValueMap,
        width: u16,
    ) -> Result<()>
    where
        DT: DrawTarget<Color = QuadColor>,
        DT::Error: Debug,
    {
        log::info!("开始绘制日期，屏幕宽度: {}", width);
        // 获取日期数据
        let year = self.get_cache_integer(cache, "datetime.year")?;
        log::debug!("获取到年份: {}", year);
        let month = self.get_cache_integer(cache, "datetime.month")?;
        log::debug!("获取到月份: {}", month);
        let day = self.get_cache_integer(cache, "datetime.day")?;
        log::debug!("获取到日期: {}", day);
        let weekday = self.get_cache_string(cache, "datetime.weekday")?;
        log::debug!("获取到星期: {}", weekday);

        // 格式化日期字符串
        let date_str = format!("{}年{}月{}日 {}", year, month, day, weekday);
        log::debug!("格式化后的日期字符串: {}", date_str);

        // 绘制日期（水平居中）
        log::debug!("调用文本渲染器绘制日期");
        text_renderer.render(
            target,
            [0, 70, width, 30],
            &date_str,
            TextAlignment::Center,
            VerticalAlignment::Center,
            None,
            None,
            FontSize::Large,
        )?;
        log::info!("日期绘制完成");

        Ok(())
    }

    /// 绘制分割线1
    fn render_divider1<DT>(&self, target: &mut DT, width: u16) -> Result<()>
    where
        DT: DrawTarget<Color = QuadColor>,
        DT::Error: Debug,
    {
        log::info!("开始绘制分割线1，屏幕宽度: {}", width);
        // 绘制水平分割线，左右两边有5px边距
        let divider = Line::new(Point::new(5, 110), Point::new((width - 5) as i32, 110))
            .into_styled(PrimitiveStyle::with_stroke(QuadColor::Black, 1));
        log::debug!("分割线1位置: 起点({}, {}), 终点({}, {}), 粗细: 1", 5, 110, width - 5, 110);

        divider.draw(target).map_err(|_| AppError::RenderError)?;
        log::info!("分割线1绘制完成");

        Ok(())
    }

    /// 绘制农历和天气
    fn render_lunar_and_weather<DT>(
        &self,
        target: &mut DT,
        text_renderer: &TextRenderer,
        icon_renderer: &IconRenderer,
        cache: &CacheKeyValueMap,
        width: u16,
    ) -> Result<()>
    where
        DT: DrawTarget<Color = QuadColor>,
        DT::Error: Debug,
    {
        log::info!("开始绘制农历和天气，屏幕宽度: {}", width);
        // 计算左侧和右侧区域宽度
        let left_width = width / 2;
        let right_width = width - left_width;
        log::debug!("左侧区域宽度: {}, 右侧区域宽度: {}", left_width, right_width);

        // 绘制农历
        log::debug!("开始绘制农历信息");
        self.render_lunar(target, text_renderer, cache, left_width)?;
        log::debug!("农历信息绘制完成");

        // 绘制竖直分割线
        log::debug!("开始绘制竖直分割线");
        let vertical_divider = Line::new(
            Point::new(left_width as i32, 120),
            Point::new(left_width as i32, 300),
        )
        .into_styled(PrimitiveStyle::with_stroke(QuadColor::Black, 1));
        log::debug!("竖直分割线位置: 起点({}, {}), 终点({}, {})
            , 粗细: 1", left_width, 120, left_width, 300);

        vertical_divider
            .draw(target)
            .map_err(|_| AppError::RenderError)?;
        log::debug!("竖直分割线绘制完成");

        // 绘制天气
        log::debug!("开始绘制天气信息");
        self.render_weather(
            target,
            text_renderer,
            icon_renderer,
            cache,
            left_width,
            right_width,
        )?;
        log::debug!("天气信息绘制完成");
        log::info!("农历和天气绘制完成");

        Ok(())
    }

    /// 绘制农历
    fn render_lunar<DT>(
        &self,
        target: &mut DT,
        text_renderer: &TextRenderer,
        cache: &CacheKeyValueMap,
        width: u16,
    ) -> Result<()> where
        DT: DrawTarget<Color = QuadColor>,
        DT::Error: Debug,
    {
        // 获取农历数据
        let ganzhi = self.get_cache_string(cache, "datetime.lunar.ganzhi")?;
        let zodiac = self.get_cache_string(cache, "datetime.lunar.zodiac")?;
        let month = self.get_cache_string(cache, "datetime.lunar.month")?;
        let jieqi = self.get_cache_string(cache, "datetime.lunar.jieqi")?;
        let festival = self.get_cache_string(cache, "datetime.lunar.festival")?;
        
        // 获取农历日
        let day = self.get_cache_string(cache, "datetime.lunar.day")?;

        // 绘制农历年份和月份
        let lunar_year_month = format!("{}年{}", ganzhi, month);
        text_renderer.render(
            target,
            [10, 120, width - 20, 25],
            &lunar_year_month,
            TextAlignment::Center,
            VerticalAlignment::Center,
            None,
            None,
            FontSize::Medium,
        )?;

        // 绘制农历日
        text_renderer.render(
            target,
            [10, 150, width - 20, 35],
            &day,
            TextAlignment::Center,
            VerticalAlignment::Center,
            None,
            None,
            FontSize::Large,
        )?;

        // 绘制节气或节日（如果有）
        if !jieqi.is_empty() {
            text_renderer.render(
                target,
                [10, 190, width - 20, 20],
                &jieqi,
                TextAlignment::Center,
                VerticalAlignment::Center,
                None,
                None,
                FontSize::Small,
            )?;
        } else if !festival.is_empty() {
            text_renderer.render(
                target,
                [10, 190, width - 20, 20],
                &festival,
                TextAlignment::Center,
                VerticalAlignment::Center,
                None,
                None,
                FontSize::Small,
            )?;
        }

        Ok(())
    }

    /// 绘制天气
    fn render_weather<DT>(
        &self,
        target: &mut DT,
        text_renderer: &TextRenderer,
        icon_renderer: &IconRenderer,
        cache: &CacheKeyValueMap,
        left_offset: u16,
        width: u16,
    ) -> Result<()>
    where
        DT: DrawTarget<Color = QuadColor>,
        DT::Error: Debug,
    {
        // 获取天气数据
        let location = self.get_cache_string(cache, "weather.location")?;
        let temperature = self.get_cache_integer(cache, "weather.temperature")?;
        let humidity = self.get_cache_integer(cache, "weather.humidity")?;

        // 绘制位置
        text_renderer.render(
            target,
            [left_offset + 10, 120, width - 20, 25],
            &location,
            TextAlignment::Center,
            VerticalAlignment::Center,
            None,
            None,
            FontSize::Medium,
        )?;

        // 绘制当前温湿度
        let temp_humidity = format!("温度：{}°C，湿度：{}%", temperature, humidity);
        text_renderer.render(
            target,
            [left_offset + 10, 150, width - 20, 30],
            &temp_humidity,
            TextAlignment::Left,
            VerticalAlignment::Top,
            None,
            None,
            FontSize::Small,
        )?;

        // 绘制3天天气预报
        for i in 0..3 {
            let day = i + 1;
            let weather_code =
                self.get_cache_string(cache, &format!("weather.condition.day{}.code", day))?;
            let temp_max =
                self.get_cache_integer(cache, &format!("weather.day{}.temp_max", day))?;
            let temp_min =
                self.get_cache_integer(cache, &format!("weather.day{}.temp_min", day))?;

            // 计算位置
            let x = left_offset + 10 + (i * (width / 3)) as u16;
            let y = 190;

            // 绘制天气图标
            icon_renderer.render(target, [x, y, 30, 30], &weather_code)?;

            // 绘制温度范围
            let temp_range = format!("{}/{}", temp_max, temp_min);
            text_renderer.render(
                target,
                [x, y + 35, width / 3 - 10, 20],
                &temp_range,
                TextAlignment::Center,
                VerticalAlignment::Center,
                None,
                None,
                FontSize::Small,
            )?;
        }

        Ok(())
    }

    /// 绘制分割线2
    fn render_divider2<DT>(&self, target: &mut DT, width: u16) -> Result<()>
    where
        DT: DrawTarget<Color = QuadColor>,
        DT::Error: Debug,
    {
        log::info!("开始绘制分割线2，屏幕宽度: {}", width);
        // 绘制水平分割线，左右两边有5px边距
        let divider = Line::new(Point::new(5, 320), Point::new((width - 5) as i32, 320))
            .into_styled(PrimitiveStyle::with_stroke(QuadColor::Black, 1));
        log::debug!("分割线2位置: 起点({}, {}), 终点({}, {}), 粗细: 1", 5, 320, width - 5, 320);

        divider.draw(target).map_err(|_| AppError::RenderError)?;
        log::info!("分割线2绘制完成");

        Ok(())
    }

    /// 绘制格言
    fn render_motto<DT>(
        &self,
        target: &mut DT,
        text_renderer: &TextRenderer,
        cache: &CacheKeyValueMap,
        width: u16,
    ) -> Result<()>
    where
        DT: DrawTarget<Color = QuadColor>,
        DT::Error: Debug,
    {
        log::info!("开始绘制格言，屏幕宽度: {}", width);
        // 获取格言数据
        let motto = self.get_cache_string(cache, "hitokoto.content")?;
        log::debug!("获取到格言内容: {}", motto);
        let author = self.get_cache_string(cache, "hitokoto.author")?;
        log::debug!("获取到格言作者: {}", author);

        // 格式化格言
        let motto_text = format!("{} —— {}", motto, author);
        log::debug!("格式化后的格言: {}", motto_text);

        // 绘制格言（垂直居中）
        log::debug!("调用文本渲染器绘制格言");
        text_renderer.render(
            target,
            [10, 340, width - 20, 120],
            &motto_text,
            TextAlignment::Center,
            VerticalAlignment::Center,
            None,
            None,
            FontSize::Medium,
        )?;
        log::info!("格言绘制完成");

        Ok(())
    }
}