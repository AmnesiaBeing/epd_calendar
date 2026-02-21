//! 渲染引擎模块
//! 负责将时间、农历、天气等信息渲染到渲染缓冲区

mod text;
mod icon;
mod layout;

pub use text::TextRenderer;
pub use icon::IconRenderer;
pub use layout::LayoutRenderer;
pub use framebuffer::Framebuffer;

/// 渲染器主结构
pub struct Renderer {
    framebuffer: Framebuffer,
    text_renderer: TextRenderer,
    icon_renderer: IconRenderer,
    layout_renderer: LayoutRenderer,
}

impl Renderer {
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            framebuffer: Framebuffer::new(width, height),
            text_renderer: TextRenderer::new(),
            icon_renderer: IconRenderer::new(),
            layout_renderer: LayoutRenderer::new(),
        }
    }

    /// 渲染时间区域
    pub fn render_time(&mut self, time: &DateTime) -> SystemResult<()> {
        use heapless::String;
        let mut time_str = String::from_capacity(5);
        let _ = write!(time_str, "{:02}:{:02}", time.hour, time.minute);

        // 在顶部20%位置渲染时间
        self.text_renderer.render(&self.framebuffer, 10, 30, &time_str)?;
        self.layout_renderer.draw_divider(&self.framebuffer, 80)?;
        Ok(())
    }

    /// 渲染农历区域
    pub fn render_lunar(&mut self, lunar: &LunarDate) -> SystemResult<()> {
        use heapless::String;
        let mut date_str = String::from_capacity(20);
        let _ = write!(date_str, "{}年{}月{}日 ", lunar.year, lunar.month, lunar.day);
        let _ = write!(date_str, "{} [{}]", lunar.zodiac, lunar.ganzhi_year);

        // 在农历区域渲染
        self.text_renderer.render(&self.framebuffer, 10, 90, &date_str)?;
        Ok(())
    }

    /// 渲染天气区域
    pub fn render_weather(&mut self, weather: &WeatherInfo) -> SystemResult<()> {
        // 渲染当前温度
        let temp = format!("{}°C", weather.current.temp);
        self.text_renderer.render(&self.framebuffer, 10, 140, &temp)?;

        // 渲染相对湿度
        let humidity = format!("{}%", weather.current.humidity);
        self.text_renderer.render(&self.framebuffer, 100, 140, &humidity)?;

        // 渲染天气图标
        self.icon_renderer.render_weather_icon(&self.framebuffer, 250, 110, weather.current.condition)?;

        Ok(())
    }

    /// 渲染格言区域
    pub fn render_quote(&mut self, quote: &str) -> SystemResult<()> {
        self.text_renderer.render(&self.framebuffer, 10, 220, quote)?;
        Ok(())
    }

    /// 获取渲染缓冲区
    pub fn framebuffer(&self) -> &Framebuffer {
        &self.framebuffer
    }
}
