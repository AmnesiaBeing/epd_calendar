use embassy_executor::Spawner;
use epd_yrd0750ryf665f60::{prelude::WaveshareDisplay as _, yrd0750ryf665f60::Epd7in5};
use linux_embedded_hal::{SpidevDevice, SysfsPin};
use lxx_calendar_common::*;
use lxx_calendar_core::main_task;
use simulated_rtc::SimulatedRtc;
use simulated_wdt::SimulatedWdt;

pub struct LinuxBuzzer;

impl BuzzerDriver for LinuxBuzzer {
    type Error = core::convert::Infallible;

    fn play_tone(&mut self, frequency: u32, duration_ms: u32) -> Result<(), Self::Error> {
        // Linux: 通过 /sys/class/pwm 驱动 pwm-beeper
        // 如果不可用，则记录日志
        // 实际实现可以使用 sysfs PWM:
        // echo 0 > /sys/class/pwm/pwmchip0/export
        // echo {frequency} > /sys/class/pwm/pwmchip0/pwm0/period
        // echo {duty} > /sys/class/pwm/pwmchip0/pwm0/duty_cycle
        // echo 1 > /sys/class/pwm/pwmchip0/pwm0/enable

        std::thread::sleep(std::time::Duration::from_millis(duration_ms as u64));

        todo!()
    }
}

pub struct LinuxWifi {
    connected: bool,
    interface: Option<String>,
}

impl LinuxWifi {
    pub fn new() -> Self {
        Self {
            connected: false,
            interface: None,
        }
    }
}

impl Default for LinuxWifi {
    fn default() -> Self {
        Self::new()
    }
}

impl WifiController for LinuxWifi {
    type Error = core::convert::Infallible;

    async fn connect_sta(&mut self, ssid: &str, password: &str) -> Result<(), Self::Error> {
        use wifi_rs::WiFi;
        use wifi_rs::prelude::*;

        info!("Linux WiFi connecting to SSID: {}", ssid);

        let mut wifi = WiFi::new(None);

        match wifi.connect(ssid, password) {
            Ok(true) => {
                info!("Linux WiFi connected successfully");
                self.connected = true;
                self.interface = Some("wlan0".to_string());
            }
            Ok(false) => {
                warn!("Linux WiFi connection failed - invalid password");
                self.connected = false;
            }
            Err(e) => {
                error!("Linux WiFi connection error: {:?}", e);
                self.connected = false;
            }
        }

        Ok(())
    }

    async fn disconnect(&mut self) -> Result<(), Self::Error> {
        use wifi_rs::WiFi;
        use wifi_rs::prelude::*;

        info!("Linux WiFi disconnecting");

        let mut wifi = WiFi::new(self.interface.as_ref().map(|s| Config {
            interface: Some(s.as_str()),
        }));

        wifi.disconnect().ok();

        self.connected = false;
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.connected
    }
}

pub struct LinuxNetwork;

impl LinuxNetwork {
    pub fn new() -> Self {
        Self
    }
}

impl Default for LinuxNetwork {
    fn default() -> Self {
        Self::new()
    }
}

impl NetworkStack for LinuxNetwork {
    type Error = core::convert::Infallible;

    fn is_link_up(&self) -> bool {
        true
    }

    async fn wait_config_up(&self) -> Result<(), Self::Error> {
        info!("Linux network waiting for config (stub)");
        Ok(())
    }

    fn is_config_up(&self) -> bool {
        true
    }
}

pub struct Platform;

impl PlatformTrait for Platform {
    type WatchdogDevice = SimulatedWdt;

    type EpdDevice = SpidevDevice;

    type AudioDevice = LinuxBuzzer;

    type RtcDevice = SimulatedRtc;

    type WifiDevice = LinuxWifi;

    type NetworkStack = LinuxNetwork;

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

        let wifi = LinuxWifi::new();
        let network = LinuxNetwork::new();

        PlatformContext {
            sys_watch_dog: wdt,
            epd: spi,
            audio,
            rtc,
            wifi,
            network,
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
