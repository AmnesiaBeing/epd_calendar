//! 构建配置管理模块

use anyhow::{Context, Result};
use std::path::PathBuf;

use crate::builder::modules::font_generator::FontSizeConfig;

/// 本地静态图标分类配置
#[derive(Debug, Clone)]
pub struct LocalIconCategoryConfig {
    /// 分类名称，如"battery"
    pub category: String,
    /// 图标目录路径，如assets/icons/battery
    pub dir: PathBuf,
    /// 生成的Rust枚举名，如"BatteryIcon"
    pub enum_name: String,
    /// 图标宽度（硬编码）
    pub width: u32,
    /// 图标高度（硬编码）
    pub height: u32,
}

/// 天气图标配置
#[derive(Debug, Clone)]
pub struct WeatherIconConfig {
    /// 天气图标根目录，如../Icons
    pub dir: PathBuf,
    /// 图标清单JSON路径，如../Icons/icons-list.json
    pub list_path: PathBuf,
    /// 生成的枚举名，如"WeatherIcon"
    pub enum_name: String,
    /// 图标宽度（硬编码）
    pub width: u32,
    /// 图标高度（硬编码）
    pub height: u32,
}

/// 构建配置
#[derive(Debug, Clone)]
pub struct BuildConfig {
    /// 输出目录
    pub output_dir: PathBuf,
    /// shared输出目录
    pub shared_output_dir: PathBuf,
    /// 句子目录
    pub sentences_dir: PathBuf,
    /// 分类配置路径
    pub categories_path: PathBuf,
    /// 字体路径
    pub font_path: PathBuf,
    /// 字体尺寸配置列表
    pub font_size_configs: Vec<FontSizeConfig>,
    /// 本地静态图标分类配置列表
    pub local_icon_categories: Vec<LocalIconCategoryConfig>,
    /// 天气图标配置
    pub weather_icon_config: WeatherIconConfig,
    /// 主布局配置路径
    pub main_layout_path: PathBuf,
}

impl BuildConfig {
    /// 加载默认配置
    pub fn load() -> Result<Self> {
        Ok(Self {
            output_dir: PathBuf::from("src/assets"),
            shared_output_dir: PathBuf::from("shared"),
            sentences_dir: PathBuf::from("../sentences-bundle/sentences"),
            categories_path: PathBuf::from("../sentences-bundle/categories.json"),
            font_path: PathBuf::from("assets/fonts/MapleMono-NF-CN-Regular.ttf"),
            font_size_configs: vec![
                FontSizeConfig::new("Small", 16),  // 小号字体 16px
                FontSizeConfig::new("Medium", 24), // 中号字体 24px
                FontSizeConfig::new("Large", 40),  // 大号字体 40px
            ],
            local_icon_categories: vec![
                LocalIconCategoryConfig {
                    category: "battery".to_string(),
                    dir: PathBuf::from("assets/icons/battery"),
                    enum_name: "BatteryIcon".to_string(),
                    width: 32,
                    height: 32,
                },
                LocalIconCategoryConfig {
                    category: "network".to_string(),
                    dir: PathBuf::from("assets/icons/network"),
                    enum_name: "NetworkIcon".to_string(),
                    width: 32,
                    height: 32,
                },
                LocalIconCategoryConfig {
                    category: "time_digit".to_string(),
                    dir: PathBuf::from("assets/icons/time_digit"),
                    enum_name: "TimeDigitIcon".to_string(),
                    width: 48,
                    height: 64,
                },
            ],
            weather_icon_config: WeatherIconConfig {
                dir: PathBuf::from("../Icons"),
                list_path: PathBuf::from("../Icons/icons-list.json"),
                enum_name: "WeatherIcon".to_string(),
                width: 64,
                height: 64,
            },
            main_layout_path: PathBuf::from("assets/layout/main.yaml"),
        })
    }

    /// 确保输出目录存在
    pub fn ensure_output_dirs(&self) -> Result<()> {
        std::fs::create_dir_all(&self.output_dir)
            .with_context(|| format!("创建输出目录失败: {}", self.output_dir.display()))?;
        Ok(())
    }
}
