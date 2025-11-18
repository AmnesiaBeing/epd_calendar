//! 构建配置管理

use anyhow::{Context, Result};
use std::path::PathBuf;

/// 构建配置
#[derive(Debug, Clone)]
pub struct BuildConfig {
    // 路径配置
    pub output_dir: PathBuf,
    pub sentences_dir: PathBuf,
    pub categories_path: PathBuf,
    pub font_path: PathBuf,
    pub weather_icons_dir: PathBuf,

    // 尺寸配置
    pub font_size: u32,
    pub weather_icon_size: u32,
}

impl BuildConfig {
    /// 加载默认配置
    pub fn load() -> Result<Self> {
        Ok(Self {
            output_dir: PathBuf::from("src/drv"),
            sentences_dir: PathBuf::from("../sentences-bundle/sentences"),
            categories_path: PathBuf::from("../sentences-bundle/categories.json"),
            font_path: PathBuf::from("assets/NotoSansMonoCJKsc-Regular.otf"),
            weather_icons_dir: PathBuf::from("../Icons"),

            font_size: 24,
            weather_icon_size: 128,
        })
    }

    /// 确保输出目录存在
    pub fn ensure_output_dirs(&self) -> Result<()> {
        std::fs::create_dir_all(&self.output_dir)
            .with_context(|| format!("创建输出目录失败: {}", self.output_dir.display()))?;
        Ok(())
    }
}
