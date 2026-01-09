/// 文件工具
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
