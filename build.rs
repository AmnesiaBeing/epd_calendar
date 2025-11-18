//! 嵌入式资源构建脚本 - 主入口

mod builder;

use anyhow::Result;

fn main() -> Result<()> {
    builder::run()
}
