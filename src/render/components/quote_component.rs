use embedded_graphics::{
    Drawable,
    prelude::{Dimensions, DrawTarget},
    primitives::Rectangle,
};
use epd_waveshare::color::QuadColor;

use crate::assets::generated_hitokoto_data::Hitokoto;

impl Drawable for &Hitokoto {
    type Color = QuadColor;

    type Output = ();

    fn draw<D>(&self, target: &mut D) -> Result<Self::Output, D::Error>
    where
        D: DrawTarget<Color = Self::Color>,
    {
        todo!()
    }
}

impl Dimensions for &Hitokoto {
    fn bounding_box(&self) -> Rectangle {
        todo!()
    }
}
