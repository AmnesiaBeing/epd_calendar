use embedded_graphics::{
    Drawable,
    prelude::{DrawTarget, Point, Primitive, Size},
    primitives::{Line, PrimitiveStyle, Rectangle},
};
use epd_waveshare::color::QuadColor;

use crate::{
    assets::generated_fonts::FontSize,
    render::components::{quote_component::HITOKOTO_RECT, time_component::TIME_RECT},
};

const X1: i32 = 5;
const X2: i32 = 795;
const X3: i32 = 300;

const Y1: i32 = TIME_RECT.top_left.y as i32
    + TIME_RECT.size.height as i32
    + FontSize::Large.pixel_size() as i32
    + 5;
const Y2: i32 = HITOKOTO_RECT.top_left.y as i32 - 10;

pub struct SeparatorComponent;

impl Drawable for SeparatorComponent {
    type Color = QuadColor;

    type Output = ();

    fn draw<D>(&self, target: &mut D) -> Result<Self::Output, D::Error>
    where
        D: DrawTarget<Color = Self::Color>,
    {
        Line::new(Point::new(X1, Y1), Point::new(X2, Y1))
            .into_styled(PrimitiveStyle::with_stroke(QuadColor::Black, 2))
            .draw(target)?;

        Line::new(Point::new(X1, Y2), Point::new(X2, Y2))
            .into_styled(PrimitiveStyle::with_stroke(QuadColor::Black, 2))
            .draw(target)?;

        Line::new(Point::new(X3, Y1 + 5), Point::new(X3, Y2 - 5))
            .into_styled(PrimitiveStyle::with_stroke(QuadColor::Black, 2))
            .draw(target)?;

        Ok(())
    }
}
