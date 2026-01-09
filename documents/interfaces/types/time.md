# 时间 (Time) 数据类型文档

## 概述

时间类型用于管理系统时间、时区和日历。本项目使用jiff库进行时间处理，支持no_std环境。

## DateTime

### 结构定义

```rust
pub struct DateTime {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
    pub weekday: u8,
    pub timezone_offset: i32,
}
```

### 字段说明

- `year` - 年份（例如：2024）
- `month` - 月份（1-12）
- `day` - 日期（1-31）
- `hour` - 小时（0-23）
- `minute` - 分钟（0-59）
- `second` - 秒（0-59）
- `weekday` - 星期（0-6，0=Monday）
- `timezone_offset` - 时区偏移（相对于UTC的秒数，例如：28800表示UTC+8）

### 使用示例

```rust
let now = DateTime {
    year: 2024,
    month: 1,
    day: 15,
    hour: 14,
    minute: 30,
    second: 0,
    weekday: 0,
    timezone_offset: 28800,
};
```

## TimeZone

### 结构定义

```rust
pub struct TimeZone {
    pub offset_seconds: i32,
    pub name: String,
    pub abbreviation: String,
}
```

### 字段说明

- `offset_seconds` - 时区偏移（相对于UTC的秒数）
- `name` - 时区名称（例如："Asia/Shanghai"）
- `abbreviation` - 时区缩写（例如："CST"）

### 使用示例

```rust
let tz = TimeZone {
    offset_seconds: 28800,
    name: "Asia/Shanghai".to_string(),
    abbreviation: "CST".to_string(),
};
```

## LunarDate

### 结构定义

```rust
pub struct LunarDate {
    pub year: i32,
    pub month: u8,
    pub day: u8,
    pub is_leap_month: bool,
    pub ganzhi_year: String,
    pub ganzhi_month: String,
    pub ganzhi_day: String,
    pub zodiac: Zodiac,
}
```

### 字段说明

- `year` - 农历年份
- `month` - 农历月份（1-12）
- `day` - 农历日期（1-30）
- `is_leap_month` - 是否为闰月
- `ganzhi_year` - 年干支（例如："甲辰"）
- `ganzhi_month` - 月干支
- `ganzhi_day` - 日干支
- `zodiac` - 生肖

### 使用示例

```rust
let lunar = LunarDate {
    year: 2023,
    month: 12,
    day: 15,
    is_leap_month: false,
    ganzhi_year: "甲辰".to_string(),
    ganzhi_month: "丙子".to_string(),
    ganzhi_day: "丁卯".to_string(),
    zodiac: Zodiac::Dragon,
};
```

## SolarTerm

### 结构定义

```rust
pub struct SolarTerm {
    pub name: String,
    pub date: DateTime,
    pub is_today: bool,
}
```

### 字段说明

- `name` - 节气名称（例如："小寒"）
- `date` - 节气日期
- `is_today` - 是否为今天

### 使用示例

```rust
let term = SolarTerm {
    name: "小寒".to_string(),
    date: DateTime { ... },
    is_today: false,
};
```

## Holiday

### 结构定义

```rust
pub struct Holiday {
    pub name: String,
    pub date: DateTime,
    pub is_today: bool,
    pub is_workday: bool,
}
```

### 字段说明

- `name` - 节假日名称（例如："元旦"）
- `date` - 节假日日期
- `is_today` - 是否为今天
- `is_workday` - 是否为工作日

### 使用示例

```rust
let holiday = Holiday {
    name: "元旦".to_string(),
    date: DateTime { ... },
    is_today: false,
    is_workday: false,
};
```

## 数据类型定义

### Zodiac

```rust
pub enum Zodiac {
    Rat,
    Ox,
    Tiger,
    Rabbit,
    Dragon,
    Snake,
    Horse,
    Goat,
    Monkey,
    Rooster,
    Dog,
    Pig,
}
```

### Weekday

```rust
pub enum Weekday {
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
    Sunday,
}
```

## 实现注意事项

### 时间计算

- 使用jiff库进行时间处理
- 时区转换：使用jiff的TimeZone功能
- 农历计算：使用sxtwl-rs库
- 节气计算：使用sxtwl-rs库
- 节假日计算：使用sxtwl-rs库

### 时间同步

- 从SNTP服务器获取网络时间
- 校准本地RTC，设置时间偏移
- 每次网络同步后更新时间
- 低电量时延长同步间隔

### 时间缓存

- 农历、节气、节假日每日计算一次
- 缓存结果到NVS存储
- 减少计算开销，降低功耗

### 时间跳变处理

- 检测时间跳变（网络同步/手动修改）
- 重新计算关联事件
- 同步更新至LPU

## 性能要求

- 时间读取延迟：≤10ms
- 时间设置延迟：≤10ms
- 时区转换延迟：≤10ms
- 农历计算延迟：≤100ms
- 节气计算延迟：≤100ms
- 节假日计算延迟：≤100ms
