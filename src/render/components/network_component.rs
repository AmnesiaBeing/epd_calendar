use embedded_graphics::{
    Drawable,
    prelude::{Dimensions, DrawTarget, Point, Size},
    primitives::Rectangle,
};
use epd_waveshare::color::QuadColor;

use crate::{
    assets::generated_network_icons::{
        NETWORK_ICON_HEIGHT, NETWORK_ICON_WIDTH, NetworkIcon, get_network_icon_data,
    },
    common::system_state::NetworkStatus,
    render::draw_binary_image,
};

const NETWORK_ICON_RECT: Rectangle = Rectangle::new(
    Point::new(10, 10),
    Size::new(NETWORK_ICON_WIDTH, NETWORK_ICON_HEIGHT),
);

impl Drawable for &NetworkStatus {
    type Color = QuadColor;

    type Output = ();

    fn draw<D>(&self, target: &mut D) -> Result<Self::Output, D::Error>
    where
        D: DrawTarget<Color = Self::Color>,
    {
        draw_binary_image(
            target,
            get_network_icon_data(if self.0 {
                NetworkIcon::Connected
            } else {
                NetworkIcon::Disconnected
            }),
            NETWORK_ICON_RECT.size,
            NETWORK_ICON_RECT.top_left,
        )
    }
}

impl Dimensions for &NetworkStatus {
    fn bounding_box(&self) -> Rectangle {
        NETWORK_ICON_RECT.clone()
    }
}
