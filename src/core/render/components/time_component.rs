use embedded_graphics::{
    Drawable,
    prelude::{DrawTarget, Point, Size},
    primitives::Rectangle,
};
use epd_waveshare::color::QuadColor;

use crate::{
    assets::{
        generated_digit_icons::{
            DIGIT_ICON_HEIGHT, DIGIT_ICON_WIDTH, DigitIcon, get_digit_icon_data,
        },
        generated_fonts::FontSize,
    },
    common::system_state::{AMPM, TimeData},
    render::{TextRenderer, draw_binary_image},
};

pub const TIME_RECT: Rectangle = Rectangle::new(Point::new(220, 5), Size::new(360, 128));

impl Drawable for TimeData {
    type Color = QuadColor;

    type Output = ();

    fn draw<D>(&self, target: &mut D) -> Result<Self::Output, D::Error>
    where
        D: DrawTarget<Color = Self::Color>,
    {
        // 使用数组映射数字到DigitIcon，避免重复的match语句
        const DIGIT_ICONS: [DigitIcon; 10] = [
            DigitIcon::Zero,
            DigitIcon::One,
            DigitIcon::Two,
            DigitIcon::Three,
            DigitIcon::Four,
            DigitIcon::Five,
            DigitIcon::Six,
            DigitIcon::Seven,
            DigitIcon::Eight,
            DigitIcon::Nine,
        ];

        // 计算小时和分钟的十位和个位
        let hour_tens = (self.hour / 10) as usize;
        let hour_ones = (self.hour % 10) as usize;
        let minute_tens = (self.minute / 10) as usize;
        let minute_ones = (self.minute % 10) as usize;

        // 计算起始位置
        let start_x = TIME_RECT.top_left.x;
        let start_y = TIME_RECT.top_left.y;

        // 准备要渲染的图标序列
        let icons = [
            DIGIT_ICONS[hour_tens],
            DIGIT_ICONS[hour_ones],
            DigitIcon::Colon,
            DIGIT_ICONS[minute_tens],
            DIGIT_ICONS[minute_ones],
        ];

        // 遍历并渲染每个图标
        for (i, &icon) in icons.iter().enumerate() {
            let x = start_x + (i as u32 * DIGIT_ICON_WIDTH) as i32;
            let point = Point::new(x, start_y);

            draw_binary_image(
                target,
                get_digit_icon_data(icon),
                Size::new(DIGIT_ICON_WIDTH as u32, DIGIT_ICON_HEIGHT as u32),
                point,
            )?;
        }

        if let Some(ampm) = self.am_pm {
            (ampm).draw(target)?;
        }

        Ok(())
    }
}

const AMPM_RECT: Rectangle = Rectangle::new(Point::new(585, 93), Size::new(40, 40));

impl Drawable for AMPM {
    type Color = QuadColor;

    type Output = ();

    fn draw<D>(&self, target: &mut D) -> Result<Self::Output, D::Error>
    where
        D: DrawTarget<Color = Self::Color>,
    {
        TextRenderer::new(FontSize::Medium, AMPM_RECT.top_left)
            .draw_text(target, if self.0 { "AM" } else { "PM" })
    }
}
