//! 通用工具函数

use std::time::Instant;

/// 进度跟踪器
pub struct ProgressTracker {
    start_time: Instant,
    current_stage: Option<String>,
}

impl ProgressTracker {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            current_stage: None,
        }
    }

    pub fn start_stage(&mut self, name: &str) {
        self.current_stage = Some(name.to_string());
        println!("cargo:warning=🚀 开始: {}", name);
    }

    pub fn complete_stage(&mut self) {
        if let Some(stage) = &self.current_stage {
            println!("cargo:warning=✅ 完成: {}", stage);
        }
    }

    pub fn update_progress(&self, current: usize, total: usize, operation: &str) {
        let percentage = (current as f32 / total as f32 * 100.0) as usize;
        println!(
            "cargo:warning=📊 {}: {}/{} ({}%)",
            operation, current, total, percentage
        );
    }

    pub fn finish_build(&self) {
        let duration = self.start_time.elapsed();
        println!("cargo:warning=🎉 构建完成! 耗时: {:.2?}", duration);
    }
}

/// 文件工具
pub mod file_utils {
    use anyhow::{Context, Result};
    use std::fs;
    use std::path::Path;

    /// 安全写入文件
    pub fn write_file(path: &Path, content: &[u8]) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, content).with_context(|| format!("写入文件失败: {}", path.display()))
    }

    /// 安全写入字符串文件
    pub fn write_string_file(path: &Path, content: &str) -> Result<()> {
        write_file(path, content.as_bytes())
    }
}

/// 字符串处理工具
pub mod string_utils {
    /// 转义字符串用于 Rust 代码
    pub fn escape_string(s: &str) -> String {
        s.replace('\\', "\\\\")
            .replace('\"', "\\\"")
            .replace('\n', "\\n")
            .replace('\r', "\\r")
            .replace('\t', "\\t")
    }
}
