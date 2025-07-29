//! 电子墨水屏驱动

use super::super::graphics::buffer::FrameBuffer;
use super::super::hal::gpio::{InputPin, OutputPin};
use super::super::utils::delay::{delay_ms, delay_ns};
use embedded_hal::digital::ErrorType;
use thiserror::Error;

/// 时序参数（单位：ns）
const TCSS: u64 = 60; // Chip select setup time
const TCSH: u64 = 65; // Chip select hold time
const TSHW: u64 = 35; // SCL "H" pulse width (Write)
const TSLW: u64 = 35; // SCL "L" pulse width (Write)
const TDCS: u64 = 20; // DC setup time
const TDCH: u64 = 20; // DC hold time

/// GPIO引脚标识
pub struct Pins<CS, DC, RES, BUSY, SCL, SDI> {
    pub cs: CS,     // 片选信号
    pub dc: DC,     // 数据/命令控制
    pub res: RES,   // 复位信号
    pub busy: BUSY, // 忙信号
    pub scl: SCL,   // 时钟信号
    pub sdi: SDI,   // 数据输入/输出
}

/// 墨水屏驱动错误
#[derive(Debug, Error)]
pub enum EpdError<E: ErrorType> {
    #[error("GPIO操作错误: {0}")]
    GpioError(E::Error),
    #[error("屏幕忙超时")]
    BusyTimeout,
}

/// 电子墨水屏驱动
pub struct Epd<CS, DC, RES, BUSY, SCL, SDI>
where
    CS: OutputPin,
    DC: OutputPin,
    RES: OutputPin,
    BUSY: InputPin,
    SCL: OutputPin,
    SDI: OutputPin,
{
    pins: Pins<CS, DC, RES, BUSY, SCL, SDI>,
}

impl<CS, DC, RES, BUSY, SCL, SDI> Epd<CS, DC, RES, BUSY, SCL, SDI>
where
    CS: OutputPin,
    DC: OutputPin,
    RES: OutputPin,
    BUSY: InputPin,
    SCL: OutputPin,
    SDI: OutputPin,
{
    /// 创建新的墨水屏驱动实例
    pub fn new(pins: Pins<CS, DC, RES, BUSY, SCL, SDI>) -> Self {
        Self { pins }
    }

    /// 初始化屏幕
    pub fn init(&mut self) -> Result<(), EpdError<Self>> {
        self.reset()?;
        self.wait_idle()?;

        // 发送初始化命令（与原C代码保持一致）
        self.send_command(0x4D)?;
        self.send_data(0x78)?;

        self.send_command(0x00)?; // Panel setting Register
        self.send_data(0x2F)?;
        self.send_data(0x29)?;

        self.send_command(0x50)?; // VCOM and DATA interval setting Register
        self.send_data(0x37)?;

        self.send_command(0x65)?; // Gate/Source Start Setting Register
        self.send_data(0x00)?;
        self.send_data(0x00)?;
        self.send_data(0x00)?;
        self.send_data(0x00)?;

        self.send_command(0xE3)?;
        self.send_data(0x88)?;

        self.send_command(0xE9)?;
        self.send_data(0x01)?;

        self.send_command(0x30)?;
        self.send_data(0x08)?;

        Ok(())
    }

    /// 复位屏幕
    pub fn reset(&mut self) -> Result<(), EpdError<Self>> {
        self.pins.res.set_low().map_err(EpdError::GpioError)?;
        delay_ms(10);
        self.pins.res.set_high().map_err(EpdError::GpioError)?;
        delay_ms(10);
        Ok(())
    }

    /// 等待屏幕空闲
    pub fn wait_idle(&mut self) -> Result<(), EpdError<Self>> {
        // 最多等待5秒
        for _ in 0..500 {
            if self.pins.busy.is_high().map_err(EpdError::GpioError)? {
                return Ok(());
            }
            delay_ms(10);
        }
        Err(EpdError::BusyTimeout)
    }

    /// 发送命令
    pub fn send_command(&mut self, cmd: u8) -> Result<(), EpdError<Self>> {
        self.spi_start()?;
        self.pins.dc.set_low().map_err(EpdError::GpioError)?;
        delay_ns(TDCS);
        self.spi_write_byte(cmd)?;
        delay_ns(TDCH);
        self.spi_stop()?;
        Ok(())
    }

    /// 发送数据
    pub fn send_data(&mut self, data: u8) -> Result<(), EpdError<Self>> {
        self.spi_start()?;
        self.pins.dc.set_high().map_err(EpdError::GpioError)?;
        delay_ns(TDCS);
        self.spi_write_byte(data)?;
        delay_ns(TDCH);
        self.spi_stop()?;
        Ok(())
    }

    /// 开启电源
    pub fn power_on(&mut self) -> Result<(), EpdError<Self>> {
        self.send_command(0x04)?;
        self.wait_idle()?;
        Ok(())
    }

    /// 更新屏幕显示
    pub fn update(&mut self) -> Result<(), EpdError<Self>> {
        self.send_command(0x12)?;
        self.send_data(0x00)?;
        delay_ms(1);
        self.wait_idle()?;
        Ok(())
    }

    /// 关闭电源
    pub fn power_off(&mut self) -> Result<(), EpdError<Self>> {
        self.send_command(0x02)?;
        self.send_data(0x00)?;
        self.wait_idle()?;
        Ok(())
    }

    /// 进入深度睡眠模式
    pub fn deep_sleep(&mut self) -> Result<(), EpdError<Self>> {
        self.send_command(0x07)?;
        self.send_data(0xA5)?;
        Ok(())
    }

    /// 清除屏幕
    pub fn clear_screen(&mut self, buffer: &FrameBuffer) -> Result<(), EpdError<Self>> {
        self.send_command(0x10)?;
        self.spi_start()?;
        self.pins.dc.set_high().map_err(EpdError::GpioError)?;
        delay_ns(TDCS);

        for &byte in buffer.iter() {
            self.spi_write_byte(byte)?;
        }

        self.spi_stop()?;
        Ok(())
    }

    /// 发送缓冲区数据到屏幕
    pub fn send_buffer(&mut self, buffer: &FrameBuffer) -> Result<(), EpdError<Self>> {
        self.send_command(0x10)?;
        self.spi_start()?;
        self.pins.dc.set_high().map_err(EpdError::GpioError)?;
        delay_ns(TDCS);

        for &byte in buffer.iter() {
            self.spi_write_byte(byte)?;
        }

        self.spi_stop()?;
        Ok(())
    }

    /// 开始SPI通信
    fn spi_start(&mut self) -> Result<(), EpdError<Self>> {
        self.pins.cs.set_low().map_err(EpdError::GpioError)?;
        delay_ns(TCSS);
        Ok(())
    }

    /// 结束SPI通信
    fn spi_stop(&mut self) -> Result<(), EpdError<Self>> {
        delay_ns(TCSH);
        self.pins.cs.set_high().map_err(EpdError::GpioError)?;
        Ok(())
    }

    /// 通过SPI写入一个字节
    fn spi_write_byte(&mut self, data: u8) -> Result<(), EpdError<Self>> {
        for i in 0..8 {
            let bit = (data >> (7 - i)) & 0x01;

            // 写入数据位
            if bit == 1 {
                self.pins.sdi.set_high().map_err(EpdError::GpioError)?;
            } else {
                self.pins.sdi.set_low().map_err(EpdError::GpioError)?;
            }

            // 时钟脉冲
            self.pins.scl.set_high().map_err(EpdError::GpioError)?;
            delay_ns(TSHW);
            self.pins.scl.set_low().map_err(EpdError::GpioError)?;
            delay_ns(TSLW);
        }
        Ok(())
    }
}

impl<CS, DC, RES, BUSY, SCL, SDI> ErrorType for Epd<CS, DC, RES, BUSY, SCL, SDI>
where
    CS: OutputPin,
    DC: OutputPin,
    RES: OutputPin,
    BUSY: InputPin,
    SCL: OutputPin,
    SDI: OutputPin,
{
    type Error = EpdError<Self>;
}
