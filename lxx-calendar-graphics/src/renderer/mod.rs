//! 渲染引擎模块
//! 负责将时间、农历、天气等信息渲染到渲染缓冲区
//!
//! 使用示例:
//! ```rust,ignore
//! let mut renderer = Renderer::<384000>::new(800, 480);
//! renderer.render_time(&time);
//! renderer.render_lunar(&lunar);
//! ```

extern crate alloc;

mod framebuffer;
mod icon;
mod text;

pub use framebuffer::{Color, Framebuffer, FramebufferError};
pub use icon::IconRenderer;
pub use text::TextRenderer;

use lxx_calendar_common::SystemResult;
use lxx_calendar_common::types::{LunarDate, WeatherInfo};

/// 日期时间信息（本地定义，用于向后兼容）
#[derive(Debug, Clone)]
pub struct DateTime {
    pub hour: u8,
    pub minute: u8,
}

/// 渲染器主结构
///
/// SIZE 参数定义帧缓冲区大小（字节）
/// 对于 800x480 单色屏幕：800 * 480 = 384000 字节
pub struct Renderer<const SIZE: usize> {
    framebuffer: Framebuffer<SIZE>,
    text_renderer: TextRenderer,
    icon_renderer: IconRenderer,
}

impl<const SIZE: usize> Renderer<SIZE> {
    pub fn new(width: u16, height: u16) -> Option<Self> {
        Framebuffer::new(width, height).map(|framebuffer| Self {
            framebuffer,
            text_renderer: TextRenderer::new(),
            icon_renderer: IconRenderer::new(),
        })
    }

    /// 渲染时间区域 (格式：HH:MM)
    pub fn render_time(&mut self, time: &DateTime) -> SystemResult<()> {
        use core::fmt::Write;
        use heapless::String;

        let mut time_str = String::<8>::new();
        write!(time_str, "{:02}:{:02}", time.hour, time.minute).map_err(|_| {
            lxx_calendar_common::SystemError::ServiceError(
                lxx_calendar_common::ServiceError::OperationFailed,
            )
        })?;

        // 在顶部区域渲染时间 (大字体)
        self.text_renderer
            .render_large(&mut self.framebuffer, 10, 30, time_str.as_str())?;

        Ok(())
    }

    /// 渲染农历区域
    pub fn render_lunar(&mut self, lunar: &LunarDate) -> SystemResult<()> {
        use core::fmt::Write;
        use heapless::String;

        let mut date_str = String::<32>::new();
        write!(
            date_str,
            "{}年{}月{}日 ",
            lunar.year, lunar.month, lunar.day
        )
        .map_err(|_| {
            lxx_calendar_common::SystemError::ServiceError(
                lxx_calendar_common::ServiceError::OperationFailed,
            )
        })?;
        write!(date_str, "{} [{}]", lunar.zodiac, lunar.ganzhi_year).map_err(|_| {
            lxx_calendar_common::SystemError::ServiceError(
                lxx_calendar_common::ServiceError::OperationFailed,
            )
        })?;

        // 在农历区域渲染
        self.text_renderer
            .render(&mut self.framebuffer, 10, 90, date_str.as_str())?;
        Ok(())
    }

    /// 渲染天气区域
    pub fn render_weather(&mut self, weather: &WeatherInfo) -> SystemResult<()> {
        use core::fmt::Write;
        use heapless::String;

        // 渲染当前温度
        let mut temp_str = String::<16>::new();
        write!(temp_str, "{}°C", weather.current.temp).map_err(|_| {
            lxx_calendar_common::SystemError::ServiceError(
                lxx_calendar_common::ServiceError::OperationFailed,
            )
        })?;
        self.text_renderer
            .render(&mut self.framebuffer, 10, 140, temp_str.as_str())?;

        // 渲染相对湿度
        let mut humidity_str = String::<16>::new();
        write!(humidity_str, "{}%", weather.current.humidity).map_err(|_| {
            lxx_calendar_common::SystemError::ServiceError(
                lxx_calendar_common::ServiceError::OperationFailed,
            )
        })?;
        self.text_renderer
            .render(&mut self.framebuffer, 100, 140, humidity_str.as_str())?;

        // 判断是否为白天（6 点 -18 点）
        // 注意：这里使用固定时间判断，实际项目中应该传入当前时间
        let is_day = true; // 默认白天，TODO: 从系统时间获取

        // 渲染天气图标
        self.icon_renderer.render_weather_icon(
            &mut self.framebuffer,
            250,
            110,
            weather.current.condition,
            is_day,
        )?;

        Ok(())
    }

    /// 渲染格言区域
    pub fn render_quote(&mut self, quote: &str) -> SystemResult<()> {
        self.text_renderer
            .render(&mut self.framebuffer, 10, 220, quote)?;
        Ok(())
    }

    /// 渲染日期信息 (公历 + 农历 + 节气 + 节日)
    pub fn render_date_info(
        &mut self,
        year: u16,
        month: u16,
        day: u16,
        weekday: &str,
        lunar_month: u8,
        lunar_day: u8,
        solar_term: Option<&str>,
        festival: Option<&str>,
    ) -> SystemResult<()> {
        use core::fmt::Write;
        use heapless::String;

        // 公历日期
        let mut date_str = String::<32>::new();
        write!(date_str, "{}-{:02}-{:02} {}", year, month, day, weekday).map_err(|_| {
            lxx_calendar_common::SystemError::ServiceError(
                lxx_calendar_common::ServiceError::OperationFailed,
            )
        })?;
        self.text_renderer
            .render(&mut self.framebuffer, 10, 55, date_str.as_str())?;

        // 农历日期
        let mut lunar_str = String::<32>::new();
        write!(lunar_str, "农历{}月{}", lunar_month, lunar_day).map_err(|_| {
            lxx_calendar_common::SystemError::ServiceError(
                lxx_calendar_common::ServiceError::OperationFailed,
            )
        })?;

        // 添加节气或节日
        if let Some(term) = solar_term {
            write!(lunar_str, " {}", term).map_err(|_| {
                lxx_calendar_common::SystemError::ServiceError(
                    lxx_calendar_common::ServiceError::OperationFailed,
                )
            })?;
        }
        if let Some(fest) = festival {
            write!(lunar_str, " {}", fest).map_err(|_| {
                lxx_calendar_common::SystemError::ServiceError(
                    lxx_calendar_common::ServiceError::OperationFailed,
                )
            })?;
        }

        self.text_renderer
            .render(&mut self.framebuffer, 10, 75, lunar_str.as_str())?;

        Ok(())
    }

    /// 渲染电池状态
    pub fn render_battery_status(
        &mut self,
        voltage: u16,
        charging: bool,
        low_battery: bool,
    ) -> SystemResult<()> {
        // 电池图标位置 (右上角)
        let bat_x = self.framebuffer.width() - 60;
        let bat_y = 10;

        // 绘制电池边框
        Self::draw_battery_icon(
            &mut self.framebuffer,
            bat_x,
            bat_y,
            voltage,
            charging,
            low_battery,
        )?;

        Ok(())
    }

    /// 获取帧缓冲区引用
    pub fn framebuffer(&self) -> &Framebuffer<SIZE> {
        &self.framebuffer
    }

    /// 获取帧缓冲区可变引用
    pub fn framebuffer_mut(&mut self) -> &mut Framebuffer<SIZE> {
        &mut self.framebuffer
    }

    /// 清除整个屏幕
    pub fn clear(&mut self, color: Color) {
        self.framebuffer.clear(color);
    }

    /// 填充整个缓冲区
    pub fn fill(&mut self, color: Color) {
        self.framebuffer.fill(color);
    }

    /// 绘制电池图标
    fn draw_battery_icon<const S: usize>(
        framebuffer: &mut Framebuffer<S>,
        x: u16,
        y: u16,
        _voltage: u16,
        _charging: bool,
        _low_battery: bool,
    ) -> SystemResult<()> {
        // 绘制电池边框
        framebuffer.draw_rectangle(x, y, 40, 20, Color::Black)?;
        // 绘制电池正极
        framebuffer.draw_rectangle(x + 40, y + 6, 4, 8, Color::Black)?;
        Ok(())
    }
}
