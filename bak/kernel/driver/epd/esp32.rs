// src/driver/display/esp.rs

use epd_waveshare::{epd7in5_yrd0750ryf665f60::Epd7in5, prelude::WaveshareDisplay};
/// ESP32平台电子墨水屏驱动模块
///
/// 本模块实现了ESP32平台下的电子墨水屏（EPD）驱动
/// 使用Waveshare EPD库和ESP32硬件SPI接口控制7.5英寸电子墨水屏
use esp_hal::{
    delay::Delay,
    gpio::{Input, InputConfig, Level, Output, OutputConfig},
    spi::{
        Mode,
        master::{Config, Spi},
    },
    time::Rate,
};

use super::DisplayDriver;
use crate::{
    common::{
        GlobalMutex,
        error::{AppError, Result},
    },
    platform::{Platform, esp32c6::Esp32Platform},
};

/// ESP32电子墨水屏驱动结构体
///
/// 封装ESP32平台的EPD驱动功能
pub struct Esp32EpdDriver {
    peripherals: &'static mut <Esp32Platform as Platform>::Peripherals,
}

impl<'p> DisplayDriver<'p> for Esp32EpdDriver {
    type P = Esp32Platform;

    fn create(peripherals: &'p mut <Self::P as Platform>::Peripherals) -> Self
    where
        Self: Sized,
    {
        Self {
            peripherals: unsafe { core::mem::transmute(peripherals) },
        }
    }

    /// 更新帧缓冲区
    ///
    /// 将图像数据写入EPD显示缓冲区
    ///
    /// # 参数
    /// - `buffer`: 图像数据缓冲区
    ///
    /// # 返回值
    /// - `Result<()>`: 更新操作结果
    async fn display_frame(&mut self, buffer: &[u8]) -> Result<()> {
        // 配置 SPI 引脚
        let sck = self.peripherals.GPIO22.reborrow();
        let sda = self.peripherals.GPIO23.reborrow();
        let cs: Output<'_> = Output::new(
            self.peripherals.GPIO21.reborrow(),
            Level::High,
            OutputConfig::default(),
        );

        // 配置 EPD 控制引脚
        let busy = Input::new(self.peripherals.GPIO18.reborrow(), InputConfig::default());
        let dc = Output::new(
            self.peripherals.GPIO20.reborrow(),
            Level::High,
            OutputConfig::default(),
        );
        let rst = Output::new(
            self.peripherals.GPIO19.reborrow(),
            Level::High,
            OutputConfig::default(),
        );

        // 获取 SPI2 实例
        let spi2 = self.peripherals.SPI2.reborrow();

        // 创建 SPI 总线
        let spi_bus = Spi::new(
            spi2,
            Config::default()
                .with_frequency(Rate::from_mhz(10))
                .with_mode(Mode::_0),
        )
        .map_err(|e| {
            log::error!("Failed to initialize SPI bus: {:?}", e);
            AppError::DisplayInit
        })?
        .with_sck(sck)
        .with_sio0(sda);

        let spi_bus = GlobalMutex::new(spi_bus.into_async());

        let mut spi_device =
            embassy_embedded_hal::shared_bus::asynch::spi::SpiDevice::new(&spi_bus, cs);

        let mut epd_delay = Delay::new();

        log::info!("EPD display initialized successfully");

        let mut epd = Epd7in5::new(&mut spi_device, busy, dc, rst, &mut epd_delay, None)
            .await
            .map_err(|e| {
                log::error!("Failed to initialize EPD display: {:?}", e);
                AppError::DisplayInit
            })?;

        epd.update_frame(&mut spi_device, buffer, &mut epd_delay)
            .await
            .map_err(|e| {
                log::error!("Failed to update frame: {:?}", e);
                AppError::DisplayUpdateFailed
            })?;

        epd.sleep(&mut spi_device, &mut epd_delay)
            .await
            .map_err(|e| {
                log::error!("Failed to sleep EPD display: {:?}", e);
                AppError::DisplaySleepFailed
            })?;

        log::debug!("EPD frame updated and displayed");
        Ok(())
    }
}
