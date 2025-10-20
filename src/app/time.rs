// //! 时间显示功能

// use super::super::graphics::{buffer::Color, text::draw_string};
// use chrono::{Local, Timelike, Weekday};
// use freetype::face::Face;

// /// 时间显示配置
// pub struct TimeConfig {
//     pub use_12hour_format: bool,
// }

// impl Default for TimeConfig {
//     fn default() -> Self {
//         Self {
//             use_12hour_format: true,
//         }
//     }
// }

// /// 绘制时间信息到缓冲区
// pub fn draw_time(
//     buffer: &mut super::super::graphics::buffer::FrameBuffer,
//     face: &mut Face,
//     config: &TimeConfig,
// ) -> Result<(), freetype::Error> {
//     let now = Local::now();

//     // 绘制日期 (年-月-日)
//     let date_str = format!("{:04}年{:02}月{:02}日", now.year(), now.month(), now.day());
//     draw_string(buffer, face, &date_str, 40, 40, Color::Black, 36)?;

//     // 绘制星期
//     let weekday = match now.weekday() {
//         Weekday::Sun => "星期日",
//         Weekday::Mon => "星期一",
//         Weekday::Tue => "星期二",
//         Weekday::Wed => "星期三",
//         Weekday::Thu => "星期四",
//         Weekday::Fri => "星期五",
//         Weekday::Sat => "星期六",
//     };
//     draw_string(buffer, face, weekday, 40, 90, Color::Black, 30)?;

//     // 绘制时间
//     if config.use_12hour_format {
//         // 12小时制
//         let hour_12 = now.hour12();
//         let time_str = format!("{:02}:{:02}", hour_12.0, now.minute());
//         draw_string(buffer, face, &time_str, 300, 200, Color::Black, 120)?;

//         // 绘制AM/PM
//         let ampm = if hour_12.1 { "AM" } else { "PM" };
//         draw_string(buffer, face, ampm, 550, 280, Color::Black, 36)?;
//     } else {
//         // 24小时制
//         let time_str = format!("{:02}:{:02}", now.hour(), now.minute());
//         draw_string(buffer, face, &time_str, 300, 200, Color::Black, 120)?;
//     }

//     Ok(())
// }
