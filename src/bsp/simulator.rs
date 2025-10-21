use embedded_hal_mock::eh1::{
    delay::NoopDelay as Delay,
    digital::{Mock as SysfsPin, State as PinState, Transaction as PinTransaction},
    spi::Mock as SPIDevice,
};
use embedded_storage_async::nor_flash::NorFlash;
use epd_waveshare::epd7in5_yrd0750ryf665f60::{Display7in5 as Epd_Display, Epd7in5 as Epd};
use epd_waveshare::prelude::WaveshareDisplay;

use sequential_storage::map::{Value, fetch_item, store_item};
use sequential_storage::{
    cache::{KeyCacheImpl, NoCache},
    map::Key,
};

use embedded_storage_std_async_mock::FlashMock;

use log::info;

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
    pub epd_spi: SPIDevice<u8>,
    pub epd: Epd<SPIDevice<u8>, SysfsPin, SysfsPin, SysfsPin, Delay>,
    pub epd_display: Epd_Display,
    pub delay: Delay,
    pub storage: KVStorage<FlashMock<32, 32, 512>, NoCache, u8>,
}

impl Board {
    pub fn new() -> Self {
        // 初始化墨水屏的接口
        let epd_busy = SysfsPin::new(&[PinTransaction::get(PinState::High)]);
        let epd_dc = SysfsPin::new(&[]);
        let epd_rst = SysfsPin::new(&[]);
        let mut epd_spi = SPIDevice::new(&[]);

        let epd = Epd::new(
            &mut epd_spi,
            epd_busy,
            epd_dc,
            epd_rst,
            &mut Delay::new(),
            None,
        )
        .expect("EPD initalize error!");

        let epd_display = Epd_Display::default();

        info!("EPD display initialized");

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
            delay: Delay::new(),
            epd_display,
            storage: kvs,
        }
    }
}
