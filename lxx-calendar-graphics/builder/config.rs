// builder/config.rs
//! 构建配置管理模块

use anyhow::{Context, Result};
use std::path::PathBuf;

use crate::builder::modules::font_generator::FontSizeConfig;

/// 图标分类配置
#[derive(Debug, Clone)]
pub struct IconCategoryConfig {
    /// 图标的分类名称，用于匹配icon_id前缀
    pub category: String,
    /// 图标文件所在的目录路径
    pub dir: PathBuf,
    /// 生成的Rust枚举类型名称
    pub enum_name: String,
    /// 图标的固定宽度（像素）
    pub width: u16,
    /// 图标的固定高度（像素）
    pub height: u16,
}

/// 天气图标配置
#[derive(Debug, Clone)]
pub struct WeatherIconConfig {
    /// 天气图标文件的根目录
    pub dir: PathBuf,
    /// 图标清单JSON文件路径，包含天气图标的元数据
    pub list_path: PathBuf,
    /// 生成的Rust枚举类型名称
    pub enum_name: String,
    /// 天气图标的固定宽度（像素）
    pub width: u16,
    /// 天气图标的固定高度（像素）
    pub height: u16,
}

/// 构建配置
#[derive(Debug, Clone)]
pub struct BuildConfig {
    /// 构建输出目录，所有生成的资源文件都会放到此目录
    pub output_dir: PathBuf,
    /// 字体文件路径，用于生成字体位图
    pub font_path: PathBuf,
    /// 字体尺寸配置列表，定义要生成的不同字体大小
    pub font_size_configs: Vec<FontSizeConfig>,
    /// 图标分类配置列表，定义不同类别的图标资源
    pub icon_categories: Vec<IconCategoryConfig>,
    /// 天气图标配置，定义天气图标的生成规则
    pub weather_icon_config: WeatherIconConfig,
    /// 主布局配置文件路径，定义界面布局结构
    pub _main_layout_path: PathBuf,
}

impl BuildConfig {
    /// 加载默认配置
    pub fn load() -> Result<Self> {
        Ok(Self {
            output_dir: PathBuf::from("src/assets"),
            font_path: PathBuf::from("assets/fonts/MapleMono-NF-CN-Regular.ttf"),
            font_size_configs: vec![
                FontSizeConfig::new("Small", 16),  // 小号字体 16px
                FontSizeConfig::new("Medium", 24), // 中号字体 24px
                FontSizeConfig::new("Large", 40),  // 大号字体 40px
            ],
            icon_categories: vec![
                IconCategoryConfig {
                    category: "battery".to_string(),
                    dir: PathBuf::from("assets/icons/battery"),
                    enum_name: "BatteryIcon".to_string(),
                    width: 32,
                    height: 32,
                },
                IconCategoryConfig {
                    category: "network".to_string(),
                    dir: PathBuf::from("assets/icons/network"),
                    enum_name: "NetworkIcon".to_string(),
                    width: 32,
                    height: 32,
                },
                IconCategoryConfig {
                    category: "time_digit".to_string(),
                    dir: PathBuf::from("assets/icons/time_digit"),
                    enum_name: "TimeDigitIcon".to_string(),
                    width: 48,
                    height: 64,
                },
            ],
            weather_icon_config: WeatherIconConfig {
                dir: PathBuf::from("./WeatherIcons"),
                list_path: PathBuf::from("./WeatherIcons/icons-list.json"),
                enum_name: "WeatherIcon".to_string(),
                width: 64,
                height: 64,
            },
            _main_layout_path: PathBuf::from("assets/layout/main.html"),
        })
    }

    /// 确保输出目录存在
    pub fn ensure_output_dirs(&self) -> Result<()> {
        std::fs::create_dir_all(&self.output_dir)
            .with_context(|| format!("创建输出目录失败: {}", self.output_dir.display()))?;
        Ok(())
    }
}
