use embedded_graphics::{Drawable, prelude::DrawTarget};
use epd_waveshare::color::QuadColor;

use crate::common::weather::WeatherData;

impl Drawable for WeatherData {
    type Color = QuadColor;

    type Output = ();

    fn draw<D>(&self, target: &mut D) -> Result<Self::Output, D::Error>
    where
        D: DrawTarget<Color = Self::Color>,
    {
        todo!()
    }
}
