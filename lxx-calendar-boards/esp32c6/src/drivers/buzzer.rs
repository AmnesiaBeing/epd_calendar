use esp_hal::{
    gpio::AnyPin,
    ledc::{self, Ledc, LowSpeed, channel},
    peripherals::Peripherals,
};
use lxx_calendar_common::BuzzerDriver;

pub struct Esp32Buzzer {
    ledc: Ledc<'static>,
    pin: AnyPin<'static>,
}

impl Esp32Buzzer {
    pub fn new(peripherals: &Peripherals) -> Self {
        Self {
            ledc: Ledc::new(unsafe { peripherals.LEDC.clone_unchecked() }),
            pin: unsafe { peripherals.GPIO7.clone_unchecked() }.into(),
        }
    }
}

impl BuzzerDriver for Esp32Buzzer {
    type Error = core::convert::Infallible;

    fn play_tone(&mut self, frequency: u32, duration_ms: u32) -> Result<(), Self::Error> {
        use esp_hal::ledc::channel::ChannelIFace;
        use esp_hal::ledc::timer::TimerIFace;
        use esp_hal::time::Rate;

        self.ledc
            .set_global_slow_clock(ledc::LSGlobalClkSource::APBClk);
        let mut timer = self.ledc.timer::<LowSpeed>(ledc::timer::Number::Timer0);

        let timer_config = ledc::timer::config::Config {
            duty: ledc::timer::config::Duty::Duty10Bit,
            clock_source: ledc::timer::LSClockSource::APBClk,
            frequency: Rate::from_hz(frequency),
        };
        timer.configure(timer_config).ok();

        let mut ch = self
            .ledc
            .channel(channel::Number::Channel0, self.pin.reborrow());
        let ch_config = channel::config::Config {
            timer: &timer,
            duty_pct: 50,
            drive_mode: esp_hal::gpio::DriveMode::PushPull,
        };
        ch.configure(ch_config).ok();

        embassy_time::Duration::from_millis(duration_ms as u64);

        ch.configure(channel::config::Config {
            timer: &timer,
            duty_pct: 0,
            drive_mode: esp_hal::gpio::DriveMode::PushPull,
        })
        .ok();

        Ok(())
    }
}
