use embassy_executor::Spawner;
use epd_yrd0750ryf665f60::{prelude::WaveshareDisplay as _, yrd0750ryf665f60::Epd7in5};
use linux_embedded_hal::{SpidevDevice, SysfsPin};
use lxx_calendar_common::*;
use lxx_calendar_core::main_task;
use simulated_wdt::SimulatedWdt;
use simulated_rtc::SimulatedRtc;

pub struct LinuxBuzzer;

impl BuzzerDriver for LinuxBuzzer {
    type Error = core::convert::Infallible;

    fn play_tone(&mut self, frequency: u32, duration_ms: u32) -> Result<(), Self::Error> {
        // Linux: 通过 /sys/class/pwm 驱动 pwm-beeper
        // 如果不可用，则记录日志
        info!("[Buzzer] Playing {}Hz for {}ms", frequency, duration_ms);

        // 实际实现可以使用 sysfs PWM:
        // echo 0 > /sys/class/pwm/pwmchip0/export
        // echo {frequency} > /sys/class/pwm/pwmchip0/pwm0/period
        // echo {duty} > /sys/class/pwm/pwmchip0/pwm0/duty_cycle
        // echo 1 > /sys/class/pwm/pwmchip0/pwm0/enable

        std::thread::sleep(std::time::Duration::from_millis(duration_ms as u64));

        Ok(())
    }

    fn stop(&mut self) -> Result<(), Self::Error> {
        info!("[Buzzer] Stopped");
        Ok(())
    }

    fn is_playing(&self) -> bool {
        false
    }
}

pub struct Platform;

impl PlatformTrait for Platform {
    type WatchdogDevice = SimulatedWdt;

    type EpdDevice = SpidevDevice;

    type AudioDevice = LinuxBuzzer;

    type RtcDevice = SimulatedRtc;

    async fn init(spawner: Spawner) -> PlatformContext<Self> {
        let epd_busy = init_gpio(101, linux_embedded_hal::sysfs_gpio::Direction::In).unwrap();
        let epd_dc = init_gpio(102, linux_embedded_hal::sysfs_gpio::Direction::Out).unwrap();
        let epd_rst = init_gpio(97, linux_embedded_hal::sysfs_gpio::Direction::Out).unwrap();

        let mut spi = SpidevDevice::open("/dev/spidev3.0").unwrap();

        let mut delay = linux_embedded_hal::Delay;
        let _epd = Epd7in5::new(&mut spi, epd_busy, epd_dc, epd_rst, &mut delay)
            .await
            .unwrap();

        let wdt = SimulatedWdt::new(5000);
        simulated_wdt::start_watchdog(&spawner, 5000);

        let audio = LinuxBuzzer;

        let mut rtc = SimulatedRtc::new();
        rtc.initialize().await.ok();

        PlatformContext {
            sys_watch_dog: wdt,
            epd: spi,
            audio,
            rtc,
        }
    }

    fn sys_reset() {
        info!("TSPI platform reset");
    }

    fn sys_stop() {
        info!("TSPI platform stop");
    }
}

fn init_gpio(
    pin: u64,
    direction: linux_embedded_hal::sysfs_gpio::Direction,
) -> Result<SysfsPin, linux_embedded_hal::sysfs_gpio::Error> {
    let gpio = SysfsPin::new(pin);
    gpio.export()?;

    while !gpio.is_exported() {}

    gpio.set_direction(direction)?;

    if direction == linux_embedded_hal::sysfs_gpio::Direction::Out {
        gpio.set_value(1)?;
    }

    Ok(gpio)
}

#[tokio::main]
async fn main() {
    let spawner = unsafe { embassy_executor::Spawner::for_current_executor().await };

    let platform_ctx = Platform::init(spawner).await;
    if let Err(e) = main_task::<Platform>(spawner, platform_ctx).await {
        error!("Main task error: {:?}", e);
    }
}
