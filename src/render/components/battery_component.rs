use embedded_graphics::{
    Drawable,
    prelude::{Dimensions, DrawTarget, Point, Size},
    primitives::Rectangle,
};
use epd_waveshare::color::QuadColor;

use crate::{
    assets::generated_battery_icons::{
        BATTERY_ICON_HEIGHT, BATTERY_ICON_WIDTH, BatteryIcon, get_battery_icon_data,
    },
    common::system_state::BatteryLevel,
    render::draw_binary_image,
};

const BATTERY_ICON_RECT: Rectangle = Rectangle::new(
    Point::new(758, 10),
    Size::new(BATTERY_ICON_WIDTH, BATTERY_ICON_HEIGHT),
);

impl Drawable for &BatteryLevel {
    type Color = QuadColor;

    type Output = ();

    fn draw<D>(&self, target: &mut D) -> Result<Self::Output, D::Error>
    where
        D: DrawTarget<Color = Self::Color>,
    {
        let battery_icon = match self {
            BatteryLevel::Level0 => BatteryIcon::Level0,
            BatteryLevel::Level1 => BatteryIcon::Level1,
            BatteryLevel::Level2 => BatteryIcon::Level2,
            BatteryLevel::Level3 => BatteryIcon::Level3,
            BatteryLevel::Level4 => BatteryIcon::Level4,
        };

        draw_binary_image(
            target,
            get_battery_icon_data(battery_icon),
            BATTERY_ICON_RECT.size,
            BATTERY_ICON_RECT.top_left,
        )
    }
}

impl Dimensions for &BatteryLevel {
    fn bounding_box(&self) -> Rectangle {
        BATTERY_ICON_RECT.clone()
    }
}
