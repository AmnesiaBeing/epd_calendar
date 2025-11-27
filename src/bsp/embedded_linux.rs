use embedded_storage_async::nor_flash::NorFlash;
use embedded_storage_std_async_mock::FlashMock;
use epd_waveshare::epd7in5_yrd0750ryf665f60::{Display7in5, Epd7in5 as Epd};
use epd_waveshare::prelude::WaveshareDisplay;
use linux_embedded_hal::{Delay, SysfsPin, sysfs_gpio::Direction};
use log::info;
use sequential_storage::map::{Value, fetch_item, store_item};
use sequential_storage::{
    cache::{KeyCacheImpl, NoCache},
    map::Key,
};

#[cfg(feature = "spi_bitbang")]
use bitbang_hal::spi_halfduplex::{SPIDevice, SpiConfig};
#[cfg(not(feature = "spi_bitbang"))]
use linux_embedded_hal::{
    SpidevDevice,
    spidev::{SpiModeFlags, Spidev, SpidevOptions},
};

pub struct KVStorage<F, C, K>
where
    F: NorFlash,
    C: KeyCacheImpl<K>,
    K: Key,
{
    flash: F,
    flash_range: core::ops::Range<u32>,
    key_cache: C,
    data_buffer: Vec<u8>,
    _key: core::marker::PhantomData<K>,
}

impl<F, C, K> KVStorage<F, C, K>
where
    F: NorFlash,
    C: KeyCacheImpl<K>,
    K: Key,
{
    /// 从存储中读取键值对（上层接口）
    pub async fn fetch_kv<'d, V: Value<'d>>(&'d mut self, key: &K) -> Option<V> {
        // 调用底层库的fetch_item，传入Board内部的资源
        fetch_item(
            &mut self.flash,
            self.flash_range.clone(),
            &mut self.key_cache,
            &mut self.data_buffer,
            key,
        )
        .await
        .unwrap()
    }

    /// 向存储中写入键值对（上层接口）
    pub async fn store_kv<'d, V: Value<'d>>(&mut self, key: &K, value: &V) {
        // 调用底层库的store_item，传入Board内部的资源
        let _ = store_item(
            &mut self.flash,
            self.flash_range.clone(),
            &mut self.key_cache,
            &mut self.data_buffer,
            key,
            value,
        )
        .await;
    }
}

pub struct Board {
    #[cfg(feature = "spi_bitbang")]
    pub epd_spi: SPIDevice<SysfsPin, SysfsPin, SysfsPin, Delay>,
    #[cfg(not(feature = "spi_bitbang"))]
    pub epd_spi: SpidevDevice,
    #[cfg(feature = "spi_bitbang")]
    pub epd: Epd<SPIDevice<SysfsPin, SysfsPin, SysfsPin, Delay>, SysfsPin, SysfsPin, SysfsPin, Delay>,
    #[cfg(not(feature = "spi_bitbang"))]
    pub epd: Epd<SpidevDevice, SysfsPin, SysfsPin, SysfsPin, Delay>,
    pub epd_display: Display7in5,
    pub delay: Delay,
    pub storage: KVStorage<FlashMock<32, 32, 512>, NoCache, u8>,
}

impl Board {
    pub fn new() -> Self {
        let epd_busy = SysfsPin::new(101);
        epd_busy.export().expect("busy export");
        while !epd_busy.is_exported() {}
        epd_busy
            .set_direction(Direction::In)
            .expect("busy Direction");

        let epd_dc = SysfsPin::new(102);
        epd_dc.export().expect("dc export");
        while !epd_dc.is_exported() {}
        epd_dc.set_direction(Direction::Out).expect("dc Direction");
        epd_dc.set_value(1).expect("dc Value set to 1");

        let epd_rst = SysfsPin::new(97);
        epd_rst.export().expect("rst export");
        while !epd_rst.is_exported() {}
        epd_rst
            .set_direction(Direction::Out)
            .expect("rst Direction");
        epd_rst.set_value(1).expect("rst Value set to 1");

        #[cfg(feature = "spi_bitbang")]
        let (mut epd_spi, mut epd) = {
            let mosi = SysfsPin::new(147);
            mosi.export().expect("miso export");
            while !mosi.is_exported() {}
            mosi.set_direction(Direction::Out).expect("MISO Direction");
            mosi.set_value(1).expect("MOSI Value set to 1");

            let sck = SysfsPin::new(146);
            sck.export().expect("sck export");
            while !sck.is_exported() {}
            sck.set_direction(Direction::Out).expect("SCK Direction");
            sck.set_value(1).expect("SCK Value set to 1");

            let cs = SysfsPin::new(150);
            cs.export().expect("cs export");
            while !cs.is_exported() {}
            cs.set_direction(Direction::Out).expect("CS Direction");
            cs.set_value(1).expect("CS Value set to 1");

            let config = SpiConfig::default();

            let mut epd_spi =
                SPIDevice::new(embedded_hal::spi::MODE_0, mosi, sck, cs, Delay, config);

            let epd = Epd::new(&mut epd_spi, epd_busy, epd_dc, epd_rst, &mut Delay, None)
                .expect("eink initalize error");

            (epd_spi, epd)
        };

        #[cfg(not(feature = "spi_bitbang"))]
        let (mut epd_spi, mut epd) = {
            let mut epd_spi = SpidevDevice::open("/dev/spidev3.0").unwrap();

            let epd = Epd::new(&mut epd_spi, epd_busy, epd_dc, epd_rst, &mut Delay, None)
                .expect("eink initalize error");

            (epd_spi, epd)
        };

        let epd_display = Display7in5::default();

        info!("E-Paper display initialized");

        // 初始化Flash外设，用于存储设置参数
        let mut flash = FlashMock::<32, 32, 512>::new("flash.bin", 4 * 1024 * 1024).unwrap();
        // 设定可操作的Flash地址
        let flash_range = 0x0000..0x3000;
        // 操作缓冲区
        let mut data_buffer = [0; 128];

        let mut kvs = KVStorage {
            flash,
            flash_range,
            data_buffer: data_buffer.to_vec(),
            key_cache: sequential_storage::cache::NoCache::new(),
            _key: core::marker::PhantomData,
        };

        Board {
            epd_spi,
            epd,
            delay: Delay,
            epd_display,
            storage: kvs,
        }
    }
}
