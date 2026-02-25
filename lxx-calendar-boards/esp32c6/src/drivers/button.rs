use alloc::boxed::Box;
use embassy_executor::Spawner;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Instant, Timer};
use esp_hal::gpio::{Input, Pull};
use esp_hal::peripherals::Peripherals;
use lxx_calendar_common::traits::button::{
    ButtonDriver, ButtonEvent, DEBOUNCE_MS, LONG_PRESS_MIN_MS,
};

/// 全局静态回调函数存储
static CALLBACK: Mutex<CriticalSectionRawMutex, Option<Box<dyn Fn(ButtonEvent) + Send>>> =
    Mutex::new(None);

pub struct Esp32Button;

impl Esp32Button {
    /// 创建按钮驱动，启动监控任务
    pub fn new(peripherals: &Peripherals, spawner: Spawner) -> Self {
        spawner
            .spawn(button_monitor_task(unsafe {
                core::mem::transmute(peripherals)
            }))
            .ok();
        Self
    }

    /// 可选：配置 GPIO0 为深度睡眠唤醒源（低电平唤醒）
    pub fn configure_wakeup(&self, _peripherals: &Peripherals) {
        // 实际使用时取消注释以下代码，并引入 esp_hal::sleep
        // use esp_hal::gpio::Level;
        // esp_hal::sleep::ext0_wakeup(peripherals.GPIO0, Level::Low);
    }
}

impl ButtonDriver for Esp32Button {
    type Error = core::convert::Infallible;

    async fn register_press_callback<F>(&mut self, callback: F) -> Result<(), Self::Error>
    where
        F: Fn(ButtonEvent) + Send + 'static,
    {
        // 将回调存入全局静态
        let mut cb_guard = CALLBACK.lock().await;
        *cb_guard = Some(Box::new(callback));
        Ok(())
    }
}

/// 按钮硬件监控任务
#[embassy_executor::task]
async fn button_monitor_task(peripherals: &'static Peripherals) {
    let button = Input::new(
        unsafe { peripherals.GPIO0.clone_unchecked() },
        esp_hal::gpio::InputConfig::default().with_pull(Pull::Up),
    );

    // 初始状态：低电平表示按下
    let mut last_state = button.is_low();
    let mut press_start: Option<Instant> = None;

    loop {
        let current_state = button.is_low();

        // 检测按下（下降沿）
        if current_state && !last_state {
            press_start = Some(Instant::now());
        }
        // 检测释放（上升沿）
        else if !current_state && last_state {
            if let Some(start) = press_start {
                let duration = Instant::now().duration_since(start);
                // 按下时间小于长按阈值视为短按
                if duration.as_millis() < LONG_PRESS_MIN_MS as u64 {
                    // 直接调用回调
                    if let Some(cb) = CALLBACK.lock().await.as_ref() {
                        cb(ButtonEvent::ShortPress);
                    }
                }
                press_start = None;
            }
        }

        // 长按持续检测
        if current_state {
            if let Some(start) = press_start {
                let duration = Instant::now().duration_since(start);
                if duration.as_millis() >= LONG_PRESS_MIN_MS as u64 {
                    if let Some(cb) = CALLBACK.lock().await.as_ref() {
                        cb(ButtonEvent::LongPress);
                    }
                    // 只触发一次
                    press_start = None;
                }
            }
        }

        last_state = current_state;
        Timer::after_millis(DEBOUNCE_MS as u64).await;
    }
}
