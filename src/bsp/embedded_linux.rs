 
#[cfg(feature = "embedded_linux")]
use linux_embedded_hal::{Delay, SPIError as Error, SysfsPin, sysfs_gpio::Direction};

#[cfg(feature = "embedded_linux")]
use epd_waveshare::epd7in5_yrd0750ryf665f60::{Display7in5, Epd7in5 as Epd};

#[cfg(feature = "embedded_linux")]
use bitbang_hal::spi_halfduplex::{SPI as SpiDevice, SpiConfig};
 
    #[cfg(feature = "embedded_linux")]
    {
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

        let config = SpiConfig::default();

        spi = SpiDevice::new(embedded_hal::spi::MODE_0, mosi, sck, delay, config);
    }
    
        #[cfg(feature = "embedded_linux")]
    {
        cs = SysfsPin::new(150);
        cs.export().expect("cs export");
        while !cs.is_exported() {}
        cs.set_direction(Direction::Out).expect("CS Direction");
        cs.set_value(1).expect("CS Value set to 1");
    }

        {
        busy = SysfsPin::new(101);
        busy.export().expect("busy export");
        while !busy.is_exported() {}
        busy.set_direction(Direction::In).expect("busy Direction");
        //busy.set_value(1).expect("busy Value set to 1");
    }

    #[cfg(feature = "embedded_linux")]
    {
        dc = SysfsPin::new(102);
        dc.export().expect("dc export");
        while !dc.is_exported() {}
        dc.set_direction(Direction::Out).expect("dc Direction");
        dc.set_value(1).expect("dc Value set to 1");
    }

    #[cfg(feature = "embedded_linux")]
    {
        rst = SysfsPin::new(97);
        rst.export().expect("rst export");
        while !rst.is_exported() {}
        rst.set_direction(Direction::Out).expect("rst Direction");
        rst.set_value(1).expect("rst Value set to 1");
    }