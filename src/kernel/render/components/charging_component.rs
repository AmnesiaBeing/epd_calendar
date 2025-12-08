use embedded_graphics::{
    Drawable,
    prelude::{DrawTarget, Point, Size},
    primitives::Rectangle,
};
use epd_waveshare::color::QuadColor;

use crate::{
    assets::generated_battery_icons::{
        BATTERY_ICON_HEIGHT, BATTERY_ICON_WIDTH, BatteryIcon, get_battery_icon_data,
    },
    common::system_state::ChargingStatus,
    render::draw_binary_image,
};

const CHARGING_ICON_RECT: Rectangle = Rectangle::new(
    Point::new(721, 10),
    Size::new(BATTERY_ICON_WIDTH, BATTERY_ICON_HEIGHT),
);

impl Drawable for ChargingStatus {
    type Color = QuadColor;

    type Output = ();

    fn draw<D>(&self, target: &mut D) -> Result<Self::Output, D::Error>
    where
        D: DrawTarget<Color = Self::Color>,
    {
        if self.0 {
            draw_binary_image(
                target,
                get_battery_icon_data(BatteryIcon::Charging),
                CHARGING_ICON_RECT.size,
                CHARGING_ICON_RECT.top_left,
            )
        } else {
            Ok(())
        }
    }
}
