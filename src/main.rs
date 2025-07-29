//! 墨水瓶渲染程序主入口

#[cfg(feature = "pc")]
use sdl2::event::Event;
#[cfg(feature = "pc")]
use sdl2::pixels::PixelFormatEnum;
#[cfg(feature = "pc")]
use sdl2::rect::Rect;

use log::{info, warn};

mod app;
mod drivers;
mod graphics;
mod hal;
mod utils;

use app::{quote::QuoteManager, time::TimeConfig, weather::WeatherInfo};
use graphics::buffer::FrameBuffer;
use graphics::text::load_font;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    #[cfg(feature = "pc")]
    env_logger::init();
    #[cfg(feature = "embedded")]
    {
        // 嵌入式平台可以使用简单的日志实现
        log::set_max_level(log::LevelFilter::Info);
    }

    info!("墨水屏渲染程序启动");

    // 加载字体
    let font_path = "./assets/fonts/wqy-zenhei.ttc";
    info!("加载字体文件: {}", font_path);
    let (library, mut face) = load_font(font_path)?;

    // 初始化帧缓冲区
    let mut buffer = FrameBuffer::new();
    buffer.clear();

    // 初始化应用组件
    let time_config = TimeConfig::default();
    let weather_info = WeatherInfo::default();
    let mut quote_manager = QuoteManager::new();
    quote_manager.random_quote();

    // 绘制内容到缓冲区
    info!("绘制内容到缓冲区");
    app::time::draw_time(&mut buffer, &mut face, &time_config)?;
    app::weather::draw_weather(&mut buffer, &mut face, &weather_info)?;
    app::quote::draw_quote(&mut buffer, &mut face, quote_manager.current_quote())?;

    #[cfg(feature = "pc")]
    {
        // PC端：使用SDL2显示
        info!("初始化SDL2显示");
        let sdl_context = sdl2::init()?;
        let video_subsystem = sdl_context.video()?;

        let window = video_subsystem
            .window(
                "墨水屏模拟器",
                graphics::buffer::WIDTH as u32,
                graphics::buffer::HEIGHT as u32,
            )
            .position_centered()
            .build()
            .map_err(|e| format!("创建窗口失败: {}", e))?;

        let mut canvas = window.into_canvas().build()?;
        let texture_creator = canvas.texture_creator();

        // 创建纹理
        let mut texture = texture_creator.create_texture(
            PixelFormatEnum::RGB24,
            sdl2::render::TextureAccess::Streaming,
            graphics::buffer::WIDTH as u32,
            graphics::buffer::HEIGHT as u32,
        )?;

        // 将缓冲区数据复制到纹理
        let rgb_data = buffer.to_rgb();
        texture.update(
            Some(Rect::new(
                0,
                0,
                graphics::buffer::WIDTH as u32,
                graphics::buffer::HEIGHT as u32,
            )),
            &rgb_data,
            graphics::buffer::WIDTH * 3,
        )?;

        // 渲染并处理事件
        canvas.copy(&texture, None, None)?;
        canvas.present();

        info!("SDL2窗口已创建，等待关闭...");
        let mut event_pump = sdl_context.event_pump()?;
        'running: loop {
            for event in event_pump.poll_iter() {
                match event {
                    Event::Quit { .. } => break 'running,
                    _ => {}
                }
            }
        }

        info!("程序退出");
    }

    #[cfg(feature = "embedded")]
    {
        // 嵌入式Linux：使用实际硬件
        use driver::epd::Pins;
        use hal::linux_gpio::LinuxGpio;

        info!("初始化嵌入式硬件");

        // 初始化GPIO引脚（与原C代码保持一致）
        let cs = LinuxGpio::new(150, "out")?; // GPIO_CS 150
        let dc = LinuxGpio::new(102, "out")?; // GPIO_DC 102
        let res = LinuxGpio::new(97, "out")?; // GPIO_RES 97
        let busy = LinuxGpio::new(101, "in")?; // GPIO_BUSY 101
        let scl = LinuxGpio::new(146, "out")?; // GPIO_SCL 146
        let sdi = LinuxGpio::new(147, "out")?; // GPIO_SDI 147

        let pins = Pins {
            cs,
            dc,
            res,
            busy,
            scl,
            sdi,
        };

        // 初始化墨水屏
        let mut epd = driver::epd::Epd::new(pins);
        epd.reset()?;
        epd.init()?;
        epd.power_on()?;

        // 发送缓冲区数据并更新屏幕
        info!("发送数据到墨水屏");
        epd.send_buffer(&buffer)?;
        epd.update()?;

        // 关闭电源并进入深度睡眠
        epd.power_off()?;
        epd.deep_sleep()?;

        info!("嵌入式程序完成");
    }

    Ok(())
}
