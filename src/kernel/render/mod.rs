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
        match cache.get(key) {
            Some(value) => match value {
                DynamicValue::String(s) => Ok(s.to_string()),
                DynamicValue::Integer(i) => Ok(i.to_string()),
                DynamicValue::Float(f) => Ok(f.to_string()),
                DynamicValue::Boolean(b) => Ok(b.to_string()),
            },
            None => {
                log::debug!("缓存键 {} 不存在，返回默认值空字符串", key);
                Ok(String::new())
            }
        }
    }

    /// 获取缓存中的整数值
    fn get_cache_integer(&self, cache: &CacheKeyValueMap, key: &str) -> Result<i32> {
        match cache.get(key) {
            Some(value) => match value {
                DynamicValue::Integer(i) => Ok(*i),
                DynamicValue::String(s) => match i32::from_str(s.as_str()) {
                    Ok(num) => Ok(num),
                    Err(_) => {
                        log::debug!("字符串转整数失败: {}, 返回默认值0", s);
                        Ok(0)
                    }
                },
                DynamicValue::Float(f) => Ok(*f as i32),
                DynamicValue::Boolean(b) => Ok(if *b { 1 } else { 0 }),
            },
            None => {
                log::debug!("缓存键 {} 不存在，返回默认值0", key);
                Ok(0)
            }
        }
    }

    /// 获取缓存中的布尔值
    fn get_cache_boolean(&self, cache: &CacheKeyValueMap, key: &str) -> Result<bool> {
        match cache.get(key) {
            Some(value) => match value {
                DynamicValue::Boolean(b) => Ok(*b),
                DynamicValue::Integer(i) => Ok(*i != 0),
                DynamicValue::String(s) => Ok(s.to_lowercase() == "true"),
                DynamicValue::Float(_) => {
                    log::debug!("浮点数转布尔值未实现，返回默认值false");
                    todo!()
                }
            },
            None => {
                log::debug!("缓存键 {} 不存在，返回默认值false", key);
                Ok(false)
            }
        }
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
        let width = display_size.width as i16;
        let _height = display_size.height as i16;

        // 创建渲染器实例
        let text_renderer = TextRenderer::new();
        let icon_renderer = IconRenderer::new();

        // 绘制时间（使用图标渲染器）
        self.render_time(target, &icon_renderer, cache, width)?;

        // 绘制日期
        self.render_date(target, &text_renderer, cache, width)?;

        // 绘制分割线1
        self.render_divider1(target, width)?;

        // 绘制农历和天气
        self.render_lunar_and_weather(target, &text_renderer, &icon_renderer, cache, width)?;

        // 绘制分割线2
        self.render_divider2(target, width)?;

        // 绘制格言
        self.render_motto(target, &text_renderer, cache, width)?;

        log::info!("布局渲染完成");
        Ok(true)
    }

    /// 绘制时间（使用图标渲染器）
    fn render_time<DT>(
        &self,
        target: &mut DT,
        icon_renderer: &IconRenderer,
        cache: &CacheKeyValueMap,
        width: i16,
    ) -> Result<()>
    where
        DT: DrawTarget<Color = QuadColor>,
        DT::Error: Debug,
    {
        log::debug!("开始绘制时间");
        // 获取时间数据
        let hour_tens = self.get_cache_integer(cache, "datetime.hour_tens")?;
        let hour_ones = self.get_cache_integer(cache, "datetime.hour_ones")?;
        let minute_tens = self.get_cache_integer(cache, "datetime.minute_tens")?;
        let minute_ones = self.get_cache_integer(cache, "datetime.minute_ones")?;

        // 计算时间图标的总宽度
        let icon_width = 72;
        let icon_height = 128;
        let total_width = icon_width * 5;

        // 计算起始X坐标（水平居中）
        let start_x = (width - total_width) / 2;
        let y = 10; // 顶部边距

        // 绘制时间图标
        icon_renderer.render(
            target,
            [start_x, y, icon_width, icon_height],
            &format!("digit_{}", hour_tens),
        )?;
        icon_renderer.render(
            target,
            [start_x + icon_width, y, icon_width, icon_height],
            &format!("digit_{}", hour_ones),
        )?;
        icon_renderer.render(
            target,
            [start_x + icon_width * 2, y, icon_width, icon_height],
            "digit_colon",
        )?;
        icon_renderer.render(
            target,
            [start_x + icon_width * 3, y, icon_width, icon_height],
            &format!("digit_{}", minute_tens),
        )?;
        icon_renderer.render(
            target,
            [start_x + icon_width * 4, y, icon_width, icon_height],
            &format!("digit_{}", minute_ones),
        )?;

        Ok(())
    }

    /// 绘制日期
    fn render_date<DT>(
        &self,
        target: &mut DT,
        text_renderer: &TextRenderer,
        cache: &CacheKeyValueMap,
        width: i16,
    ) -> Result<()>
    where
        DT: DrawTarget<Color = QuadColor>,
        DT::Error: Debug,
    {
        log::debug!("开始绘制日期");
        // 获取日期数据
        let year = self.get_cache_integer(cache, "datetime.year")?;
        let month = self.get_cache_integer(cache, "datetime.month")?;
        let day = self.get_cache_integer(cache, "datetime.day")?;
        let weekday = self.get_cache_string(cache, "datetime.weekday")?;

        // 格式化日期字符串
        let date_str = format!("{}年{}月{}日 {}", year, month, day, weekday);

        // 绘制日期（水平居中）
        text_renderer.render(
            target,
            [0, 10 + 128, width, 40],
            &date_str,
            TextAlignment::Center,
            VerticalAlignment::Center,
            None,
            None,
            FontSize::Large,
        )?;

        Ok(())
    }

    /// 绘制分割线1
    fn render_divider1<DT>(&self, target: &mut DT, width: i16) -> Result<()>
    where
        DT: DrawTarget<Color = QuadColor>,
        DT::Error: Debug,
    {
        let y = 10 + 128 + 40;
        // 绘制水平分割线，左右两边有15px边距
        let divider = Line::new(Point::new(15, y), Point::new((width - 15) as i32, y))
            .into_styled(PrimitiveStyle::with_stroke(QuadColor::Black, 1));

        divider.draw(target).map_err(|_| AppError::RenderError)?;

        Ok(())
    }

    /// 绘制农历和天气
    fn render_lunar_and_weather<DT>(
        &self,
        target: &mut DT,
        text_renderer: &TextRenderer,
        icon_renderer: &IconRenderer,
        cache: &CacheKeyValueMap,
        width: i16,
    ) -> Result<()>
    where
        DT: DrawTarget<Color = QuadColor>,
        DT::Error: Debug,
    {
        log::debug!("开始绘制农历和天气");
        // 计算左侧和右侧区域宽度
        let left_width = width / 2;
        let right_width = width - left_width;

        // 绘制农历
        self.render_lunar(target, text_renderer, cache, left_width)?;

        // 绘制竖直分割线
        let vertical_divider = Line::new(
            Point::new(left_width as i32, 10 + 128 + 40 + 10),
            Point::new(left_width as i32, 480 - 120 - 10),
        )
        .into_styled(PrimitiveStyle::with_stroke(QuadColor::Black, 1));

        vertical_divider
            .draw(target)
            .map_err(|_| AppError::RenderError)?;

        // 绘制天气
        self.render_weather(
            target,
            text_renderer,
            icon_renderer,
            cache,
            left_width,
            right_width,
        )?;

        Ok(())
    }

    /// 绘制格言
    fn render_motto<DT>(
        &self,
        target: &mut DT,
        text_renderer: &TextRenderer,
        cache: &CacheKeyValueMap,
        width: i16,
    ) -> Result<()>
    where
        DT: DrawTarget<Color = QuadColor>,
        DT::Error: Debug,
    {
        log::debug!("开始绘制格言");
        // 获取格言数据
        let motto = self.get_cache_string(cache, "hitokoto.content")?;
        let who = self.get_cache_string(cache, "hitokoto.author")?;
        let from = self.get_cache_string(cache, "hitokoto.from")?;

        // 格式化格言
        let motto_text = format!("{} —— {}《{}》", motto, who, from);

        // 绘制格言（垂直居中）
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

        Ok(())
    }

    /// 绘制农历
    fn render_lunar<DT>(
        &self,
        target: &mut DT,
        text_renderer: &TextRenderer,
        cache: &CacheKeyValueMap,
        width: i16,
    ) -> Result<()>
    where
        DT: DrawTarget<Color = QuadColor>,
        DT::Error: Debug,
    {
        log::debug!("开始绘制农历");
        // 获取农历数据
        let ganzhi = self.get_cache_string(cache, "datetime.lunar.ganzhi")?;
        let zodiac = self.get_cache_string(cache, "datetime.lunar.zodiac")?;
        let month = self.get_cache_string(cache, "datetime.lunar.month")?;
        let jieqi = self.get_cache_string(cache, "datetime.lunar.jieqi")?;
        let festival = self.get_cache_string(cache, "datetime.lunar.festival")?;

        // 获取农历日
        let day = self.get_cache_string(cache, "datetime.lunar.day")?;

        // 绘制农历年份和月份
        let lunar_year_month = format!("{}{}年{}月", ganzhi, zodiac, month);
        text_renderer.render(
            target,
            [10, 10 + 128 + 40, width - 20, 24],
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
            [10, 10 + 128 + 40 + 24, width - 20, 40],
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
                [10 + 40, 10 + 128 + 40 + 24, width - 20, 20],
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
                [10 + 40, 10 + 128 + 40 + 24, width - 20, 20],
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
        left_offset: i16,
        width: i16,
    ) -> Result<()>
    where
        DT: DrawTarget<Color = QuadColor>,
        DT::Error: Debug,
    {
        log::debug!("开始绘制天气");
        // 获取天气数据
        let location = self.get_cache_string(cache, "weather.location")?;
        let temperature = self.get_cache_string(cache, "weather.sensor.temperature")?;
        let humidity = self.get_cache_string(cache, "weather.sensor.humidity")?;
        let valid = self.get_cache_boolean(cache, "weather.valid")?;

        if !valid {
            log::debug!("天气数据无效，跳过绘制天气预报");
        }

        // 绘制位置
        text_renderer.render(
            target,
            [left_offset + 10, 10 + 128 + 40, width - 20, 25],
            &location,
            TextAlignment::Left,
            VerticalAlignment::Center,
            None,
            None,
            FontSize::Medium,
        )?;

        // 绘制当前温湿度
        let temp_humidity = format!("温度：{}°C，湿度：{}%", temperature, humidity);
        text_renderer.render(
            target,
            [left_offset + 10, 10 + 128 + 40, width - 20, 25],
            &temp_humidity,
            TextAlignment::Right,
            VerticalAlignment::Center,
            None,
            None,
            FontSize::Medium,
        )?;

        if valid {
            // 绘制3天天气预报
            for i in 0..3 {
                let day = i + 1;
                let weather_code =
                    self.get_cache_string(cache, &format!("weather.condition.day{}.code", day))?;
                let temp_max =
                    self.get_cache_integer(cache, &format!("weather.day{}.temp_max", day))?;
                let temp_min =
                    self.get_cache_integer(cache, &format!("weather.day{}.temp_min", day))?;

                if weather_code.is_empty() {
                    log::debug!("第{}天天气预报数据不完整，跳过绘制", day);
                    continue;
                }

                // 计算位置
                let x = left_offset + 10 + (i * (width / 3)) as i16;
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
        }

        Ok(())
    }

    /// 绘制分割线2
    fn render_divider2<DT>(&self, target: &mut DT, width: i16) -> Result<()>
    where
        DT: DrawTarget<Color = QuadColor>,
        DT::Error: Debug,
    {
        let y = 480 - 120 - 10;
        log::debug!("开始绘制分割线2");
        // 绘制水平分割线，左右两边有15px边距
        let divider = Line::new(Point::new(15, y), Point::new((width - 15) as i32, y))
            .into_styled(PrimitiveStyle::with_stroke(QuadColor::Black, 1));

        divider.draw(target).map_err(|_| AppError::RenderError)?;

        Ok(())
    }
}
