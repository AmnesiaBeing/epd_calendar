//! 嵌入式Linux平台的GPIO实现

use super::gpio::{InputPin, OutputPin};
use std::fs::File;
use std::io::{self, Write};
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GpioError {
    #[error("IO错误: {0}")]
    Io(#[from] io::Error),
    #[error("GPIO导出失败")]
    ExportFailed,
    #[error("GPIO方向设置失败")]
    DirectionFailed,
}

impl embedded_hal::digital::Error for GpioError {}

/// Linux GPIO引脚
pub struct LinuxGpio {
    pin: u32,
    value_file: File,
}

impl LinuxGpio {
    /// 创建新的GPIO引脚实例
    pub fn new(pin: u32, direction: &str) -> Result<Self, GpioError> {
        // 导出GPIO
        let export_path = Path::new("/sys/class/gpio/export");
        let mut export_file = File::create(export_path)?;
        writeln!(export_file, "{}", pin)?;

        // 等待sysfs节点创建
        std::thread::sleep(std::time::Duration::from_millis(10));

        // 设置方向
        let dir_path = format!("/sys/class/gpio/gpio{}/direction", pin);
        let mut dir_file = File::create(Path::new(&dir_path))?;
        writeln!(dir_file, "{}", direction)?;

        // 打开value文件
        let value_path = format!("/sys/class/gpio/gpio{}/value", pin);
        let value_file = File::options()
            .read(true)
            .write(true)
            .open(Path::new(&value_path))?;

        Ok(Self { pin, value_file })
    }
}

impl Drop for LinuxGpio {
    fn drop(&mut self) {
        // 取消导出GPIO
        let _ =
            File::create("/sys/class/gpio/unexport").and_then(|mut f| writeln!(f, "{}", self.pin));
    }
}

impl OutputPin for LinuxGpio {
    fn set_high(&mut self) -> Result<(), Self::Error> {
        self.value_file.seek(io::SeekFrom::Start(0))?;
        writeln!(self.value_file, "1")?;
        Ok(())
    }

    fn set_low(&mut self) -> Result<(), Self::Error> {
        self.value_file.seek(io::SeekFrom::Start(0))?;
        writeln!(self.value_file, "0")?;
        Ok(())
    }
}

impl InputPin for LinuxGpio {
    fn is_high(&self) -> Result<bool, Self::Error> {
        let mut buf = String::new();
        let mut file = File::open(format!("/sys/class/gpio/gpio{}/value", self.pin))?;
        std::io::Read::read_to_string(&mut file, &mut buf)?;
        Ok(buf.trim() == "1")
    }
}

impl embedded_hal::digital::ErrorType for LinuxGpio {
    type Error = GpioError;
}
