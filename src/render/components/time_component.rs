use embedded_graphics::{Drawable, prelude::DrawTarget};
use epd_waveshare::color::QuadColor;

use crate::common::system_state::TimeData;

impl Drawable for TimeData {
    type Color = QuadColor;

    type Output = ();

    fn draw<D>(&self, target: &mut D) -> Result<Self::Output, D::Error>
    where
        D: DrawTarget<Color = Self::Color>,
    {
        todo!()
    }
}
