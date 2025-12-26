// builder/mod.rs
//! 资源构建主模块

pub mod config;
pub mod modules;
pub mod utils;

use anyhow::{Result, anyhow};
use utils::progress::ProgressTracker;

/// 运行完整的资源构建流程
pub fn run() -> Result<()> {
    // 加载构建配置
    let config = config::BuildConfig::load().map_err(|e| anyhow!("加载构建配置失败：{}", e))?;

    // 配置增量编译触发
    configure_incremental_build(&config);

    let mut progress = ProgressTracker::new();

    progress.start_stage("初始化构建环境");
    config.ensure_output_dirs()?;
    progress.complete_stage();

    // 0. 处理格言数据
    progress.start_stage("处理格言数据");
    modules::hitokoto::build(&config, &progress)?;
    progress.complete_stage();

    // 1. 生成字体集（严格按顺序执行）
    progress.start_stage("生成字体集");
    modules::font_generator::build(&config, &progress)?;
    progress.complete_stage();

    // 2. 生成图标（严格按顺序执行）
    progress.start_stage("生成图标");
    modules::icon_generator::build(&config, &progress)?;
    progress.complete_stage();

    // 3. 处理布局文件（严格按顺序执行）
    progress.start_stage("处理布局文件");
    modules::layout_processor::build(&config, &progress)?;
    progress.complete_stage();

    progress.finish_build();
    Ok(())
}

/// 配置增量编译触发
fn configure_incremental_build(_config: &config::BuildConfig) {
    println!("cargo::rerun-if-changed=build.rs");
    println!("cargo::rerun-if-changed=builder/");
    println!("cargo::rerun-if-changed=assets/");
}
