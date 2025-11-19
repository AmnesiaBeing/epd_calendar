//! 资源构建主模块

pub mod config;
pub mod modules;
pub mod utils;

use anyhow::Result;
use utils::ProgressTracker;

/// 运行完整的资源构建流程
pub fn run() -> Result<()> {
    let config = config::BuildConfig::load()?;
    let mut progress = ProgressTracker::new();

    progress.start_stage("初始化构建环境");
    config.ensure_output_dirs()?;
    progress.complete_stage();

    progress.start_stage("处理格言数据");
    modules::hitokoto::build(&config, &progress)?;
    progress.complete_stage();

    progress.start_stage("处理天气图标");
    modules::weather_icons::build(&config, &progress)?;
    progress.complete_stage();

    progress.start_stage("处理电池&充电图标");
    modules::battery_icons::build(&config, &progress)?;
    progress.complete_stage();

    progress.finish_build();
    Ok(())
}
