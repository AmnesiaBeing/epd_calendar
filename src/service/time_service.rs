// src/service/time_service.rs
use crate::common::error::{AppError, Result};
use crate::common::types::{LunarData, TimeData};
use crate::driver::time_source::{DefaultTimeSource, TimeSource};
use chrono::{DateTime, Datelike, Local, Timelike};

pub struct TimeService {
    time_source: DefaultTimeSource,
    is_24_hour: bool,
    temperature_celsius: bool,
}

impl TimeService {
    pub fn new(
        time_source: DefaultTimeSource,
        is_24_hour: bool,
        temperature_celsius: bool,
    ) -> Self {
        Self {
            time_source,
            is_24_hour,
            temperature_celsius,
        }
    }

    pub async fn get_current_time(&self) -> Result<TimeData> {
        let datetime = self
            .time_source
            .now()
            .await
            .map_err(|_| AppError::TimeError)?;

        // 获取农历信息
        // let lunar_data = self.calculate_lunar_data(&datetime)?;

        Ok(TimeData {
            hour: datetime.hour() as u8,
            minute: datetime.minute() as u8,
            is_24_hour: self.is_24_hour,
            date_string: self.format_date(&datetime),
            weekday: self.get_weekday_chinese(datetime.weekday()),
            // holiday: self.get_holiday(datetime.year(), datetime.month(), datetime.day()),
            // lunar: lunar_data,
        })
    }

    pub fn set_24_hour_format(&mut self, enabled: bool) {
        self.is_24_hour = enabled;
    }

    pub fn set_temperature_celsius(&mut self, enabled: bool) {
        self.temperature_celsius = enabled;
    }

    fn format_date(&self, datetime: &DateTime<Local>) -> String {
        format!(
            "{:04}-{:02}-{:02}",
            datetime.year(),
            datetime.month(),
            datetime.day()
        )
    }

    fn get_weekday_chinese(&self, weekday: chrono::Weekday) -> String {
        match weekday {
            chrono::Weekday::Mon => "周一".to_string(),
            chrono::Weekday::Tue => "周二".to_string(),
            chrono::Weekday::Wed => "周三".to_string(),
            chrono::Weekday::Thu => "周四".to_string(),
            chrono::Weekday::Fri => "周五".to_string(),
            chrono::Weekday::Sat => "周六".to_string(),
            chrono::Weekday::Sun => "周日".to_string(),
        }
    }

    // fn get_holiday(&self, year: i32, month: u32, day: u32) -> Option<String> {
    //     // 简化的节假日判断（实际应该使用更完整的节假日库）
    //     match (month, day) {
    //         (1, 1) => Some("元旦".to_string()),
    //         (5, 1) => Some("劳动节".to_string()),
    //         (10, 1) => Some("国庆节".to_string()),
    //         _ => None,
    //     }
    // }

    // fn calculate_lunar_data(&self, datetime: &DateTime<Local>) -> Result<LunarData> {
    //     let year = datetime.year() as i16;
    //     let month = datetime.month() as u8;
    //     let day = datetime.day() as u8;

    //     // 使用sxtwl-rs库计算农历
    //     let lunar_date = self
    //         .lunar
    //         .solar_to_lunar(year, month, day)
    //         .map_err(|_| AppError::TimeError)?;

    //     Ok(LunarData {
    //         year_name: self.get_ganzhi_year(lunar_date.year),
    //         zodiac: self.get_zodiac(lunar_date.year),
    //         month: self.get_lunar_month_name(lunar_date.month, lunar_date.is_leap),
    //         day: self.get_lunar_day_name(lunar_date.day),
    //         solar_term: self.get_solar_term(year, month, day),
    //         suitable: vec!["宜事1".to_string(), "宜事2".to_string()], // 简化
    //         avoid: vec!["忌事1".to_string(), "忌事2".to_string()],    // 简化
    //     })
    // }

    // fn get_ganzhi_year(&self, year: i16) -> String {
    //     // 简化的干支年计算（实际应该使用sxtwl-rs的完整功能）
    //     let ganzhi_list = [
    //         "甲子", "乙丑", "丙寅", "丁卯", "戊辰", "己巳", "庚午", "辛未", "壬申", "癸酉", "甲戌",
    //         "乙亥", "丙子", "丁丑", "戊寅", "己卯", "庚辰", "辛巳", "壬午", "癸未", "甲申", "乙酉",
    //         "丙戌", "丁亥", "戊子", "己丑", "庚寅", "辛卯", "壬辰", "癸巳", "甲午", "乙未", "丙申",
    //         "丁酉", "戊戌", "己亥", "庚子", "辛丑", "壬寅", "癸卯", "甲辰", "乙巳", "丙午", "丁未",
    //         "戊申", "己酉", "庚戌", "辛亥", "壬子", "癸丑", "甲寅", "乙卯", "丙辰", "丁巳", "戊午",
    //         "己未", "庚申", "辛酉", "壬戌", "癸亥",
    //     ];

    //     let index = ((year - 4) % 60) as usize;
    //     ganzhi_list.get(index).unwrap_or(&"未知").to_string()
    // }

    // fn get_zodiac(&self, year: i16) -> String {
    //     let zodiacs = [
    //         "鼠", "牛", "虎", "兔", "龙", "蛇", "马", "羊", "猴", "鸡", "狗", "猪",
    //     ];
    //     let index = ((year - 4) % 12) as usize;
    //     zodiacs.get(index).unwrap_or(&"未知").to_string()
    // }

    // fn get_lunar_month_name(&self, month: u8, is_leap: bool) -> String {
    //     let months = [
    //         "正月", "二月", "三月", "四月", "五月", "六月", "七月", "八月", "九月", "十月", "冬月",
    //         "腊月",
    //     ];
    //     let name = months.get((month - 1) as usize).unwrap_or(&"未知月");
    //     if is_leap {
    //         format!("闰{}", name)
    //     } else {
    //         name.to_string()
    //     }
    // }

    // fn get_lunar_day_name(&self, day: u8) -> String {
    //     let days = [
    //         "初一", "初二", "初三", "初四", "初五", "初六", "初七", "初八", "初九", "初十", "十一",
    //         "十二", "十三", "十四", "十五", "十六", "十七", "十八", "十九", "二十", "廿一", "廿二",
    //         "廿三", "廿四", "廿五", "廿六", "廿七", "廿八", "廿九", "三十",
    //     ];
    //     days.get((day - 1) as usize).unwrap_or(&"未知").to_string()
    // }

    // fn get_solar_term(&self, year: i16, month: u8, day: u8) -> Option<String> {
    //     // 简化的节气判断（实际应该使用sxtwl-rs的完整节气功能）
    //     let terms = [
    //         (2, 4, "立春"),
    //         (2, 19, "雨水"),
    //         (3, 5, "惊蛰"),
    //         (3, 20, "春分"),
    //         (4, 5, "清明"),
    //         (4, 20, "谷雨"),
    //         (5, 5, "立夏"),
    //         (5, 21, "小满"),
    //         (6, 6, "芒种"),
    //         (6, 21, "夏至"),
    //         (7, 7, "小暑"),
    //         (7, 23, "大暑"),
    //         (8, 7, "立秋"),
    //         (8, 23, "处暑"),
    //         (9, 7, "白露"),
    //         (9, 23, "秋分"),
    //         (10, 8, "寒露"),
    //         (10, 23, "霜降"),
    //         (11, 7, "立冬"),
    //         (11, 22, "小雪"),
    //         (12, 7, "大雪"),
    //         (12, 22, "冬至"),
    //         (1, 5, "小寒"),
    //         (1, 20, "大寒"),
    //     ];

    //     for &(m, d, term) in &terms {
    //         if month == m && day == d {
    //             return Some(term.to_string());
    //         }
    //     }

    //     None
    // }
}
