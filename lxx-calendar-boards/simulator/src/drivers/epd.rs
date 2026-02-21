use embedded_hal_mock::eh1::{delay::NoopDelay, digital::no_pin::NoPin, spi::no_spi::NoSpi};
use epd_yrd0750ryf665f60::{prelude::WaveshareDisplay as _, yrd0750ryf665f60::Epd7in5};

pub async fn init_epd() -> Epd7in5<NoSpi, NoPin, NoPin, NoPin, NoopDelay> {
    let busy = NoPin::high();
    let dc = NoPin::low();
    let rst = NoPin::low();
    let mut delay = NoopDelay::new();

    let epd = Epd7in5::new(&mut NoSpi::new(), busy, dc, rst, &mut delay)
        .await
        .unwrap();

    epd
}
