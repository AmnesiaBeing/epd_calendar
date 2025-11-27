// src/common/config.rs
use embedded_graphics::geometry::{Point, Size};
use embedded_graphics::primitives::Rectangle;

pub struct LayoutConfig;

impl LayoutConfig {
    pub const DISPLAY_WIDTH: u32 = 800;
    pub const DISPLAY_HEIGHT: u32 = 480;
    pub const MAX_PARTIAL_REFRESHES: u32 = 30;

    // 区域定义
    pub const TIME_REGION: Rectangle = Rectangle::new(Point::new(50, 80), Size::new(200, 60));

    pub const DATE_REGION: Rectangle = Rectangle::new(Point::new(50, 150), Size::new(300, 30));

    pub const WEATHER_REGION: Rectangle = Rectangle::new(Point::new(500, 80), Size::new(250, 120));

    pub const QUOTE_REGION: Rectangle = Rectangle::new(Point::new(50, 200), Size::new(700, 40));

    pub const STATUS_REGION: Rectangle = Rectangle::new(Point::new(700, 20), Size::new(80, 40));

    // 字体大小定义
    pub const TIME_FONT_SIZE: u32 = 48;
    pub const TEXT_MEDIUM_SIZE: u32 = 24;
    pub const TEXT_SMALL_SIZE: u32 = 16;
}
