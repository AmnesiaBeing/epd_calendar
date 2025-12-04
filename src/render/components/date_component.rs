use alloc::string::ToString;
use embedded_graphics::{
    Drawable,
    prelude::{DrawTarget, Point, Size},
    primitives::Rectangle,
};
use epd_waveshare::color::QuadColor;

use crate::{
    assets::generated_fonts::FontSize, common::system_state::DateData, render::TextRenderer,
};

const DATE_RECT: Rectangle = Rectangle::new(Point::new(300, 135), Size::new(200, 40));
const WEEK_RECT: Rectangle = Rectangle::new(Point::new(520, 135), Size::new(120, 40));
const HOLIDAY_RECT: Rectangle = Rectangle::new(Point::new(650, 135), Size::new(150, 24));

impl Drawable for DateData {
    type Color = QuadColor;

    type Output = ();

    fn draw<D>(&self, target: &mut D) -> Result<Self::Output, D::Error>
    where
        D: DrawTarget<Color = Self::Color>,
    {
        let date_str = self.day.to_string();
        let week_str = self.week.to_string();

        let mut large_text_renderer = TextRenderer::new(FontSize::Large, DATE_RECT.top_left);
        large_text_renderer.draw_text(target, &date_str)?;

        large_text_renderer.move_to(WEEK_RECT.top_left);
        large_text_renderer.draw_text(target, &week_str)?;

        if let Some(holiday) = &self.holiday {
            let holiday_str = holiday.to_string();
            let mut small_text_renderer = TextRenderer::new(FontSize::Small, HOLIDAY_RECT.top_left);
            small_text_renderer.draw_text(target, &holiday_str)?;
        }

        Ok(())
    }
}
