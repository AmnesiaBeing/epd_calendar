use bitbang_hal::spi_halfduplex::{SPIDevice, SpiConfig};
use epd_waveshare::epd7in5_yrd0750ryf665f60::{Display7in5, Epd7in5 as Epd};
use epd_waveshare::prelude::WaveshareDisplay;
use linux_embedded_hal::{Delay, SysfsPin, sysfs_gpio::Direction};

pub struct Board {
    // pub epd_busy: SysfsPin,
    // pub epd_dc: SysfsPin,
    // pub epd_rst: SysfsPin,
    pub epd_spi: SPIDevice<SysfsPin, SysfsPin, SysfsPin, Delay>,
    pub epd:
        Epd<SPIDevice<SysfsPin, SysfsPin, SysfsPin, Delay>, SysfsPin, SysfsPin, SysfsPin, Delay>,
    pub epd_display: Display7in5,
    pub delay: Delay,
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

        let mosi = SysfsPin::new(147);
        mosi.export().expect("miso export");
        while !mosi.is_exported() {}
        mosi.set_direction(Direction::In).expect("CS Direction");
        mosi.set_value(1).expect("CS Value set to 1");

        let sck = SysfsPin::new(146);
        sck.export().expect("miso export");
        while !sck.is_exported() {}
        sck.set_direction(Direction::In).expect("CS Direction");
        sck.set_value(1).expect("CS Value set to 1");

        let cs = SysfsPin::new(150);
        cs.export().expect("cs export");
        while !cs.is_exported() {}
        cs.set_direction(Direction::Out).expect("CS Direction");
        cs.set_value(1).expect("CS Value set to 1");

        let config = SpiConfig::default();

        let mut epd_spi = SPIDevice::new(embedded_hal::spi::MODE_0, mosi, sck, cs, Delay, config);

        let epd = Epd::new(&mut epd_spi, epd_busy, epd_dc, epd_rst, &mut Delay, None)
            .expect("eink initalize error");

        let epd_display = Display7in5::default();

        Board {
            epd_spi,
            epd,
            delay: Delay,
            epd_display,
        }
    }
}
