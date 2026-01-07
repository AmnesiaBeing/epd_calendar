// src/driver/key/esp32.rs
//! ESP32 按键驱动（中断触发 + 定时器判断长短按）

use embassy_sync_07::signal::Signal;
use esp_hal::{
    gpio::{Input, InputConfig},
    interrupt::{self},
    peripherals::Interrupt,
    timer::Timer,
};

use super::{KeyDriver, KeyEvent, LONG_PRESS_THRESHOLD_MS};
use crate::{common::GlobalMutex, platform::Platform};
use crate::{
    common::error::{AppError, Result},
    platform::esp32::Esp32Platform,
};

/// ESP32 按键驱动结构体
pub struct Esp32KeyDriver<'a> {
    pin: Input<'a>,
}

impl<'a> KeyDriver<'a> for Esp32KeyDriver<'a> {
    type P = Esp32Platform;

    fn create(
        peripherals: &'a mut <Self::P as Platform>::Peripherals,
        short_press_sig: Signal<GlobalMutex<KeyEvent>, KeyEvent>,
        long_press_sig: Signal<GlobalMutex<KeyEvent>, KeyEvent>,
    ) -> Result<Self> {
        // 配置按键引脚
        let key_pin = Input::new(peripherals.GPIO9.reborrow(), InputConfig::default());

        // 初始化共享状态
        critical_section::with(|cs| {
            *SHARED_STATE.borrow_ref_mut(cs) = Some(SharedState {
                short_press_sig: unsafe { core::mem::transmute(short_press_sig) },
                long_press_sig: unsafe { core::mem::transmute(long_press_sig) },
                press_start_time: None,
            });
        });

        Ok(Self { pin: key_pin })
    }

    fn start(&mut self) -> Result<()> {
        // 配置引脚中断：下降沿（按键按下）、上升沿（按键松开）
        self.pin
            .set_interrupt_type(esp_hal::gpio::InterruptType::EdgeBoth)
            .map_err(|_| AppError::KeyInitFailed)?;

        // 注册中断处理函数
        interrupt::enable(
            self.pin.interrupt(),
            interrupt::Priority::Priority3,
            handle_key_interrupt,
        )
        .map_err(|_| AppError::KeyInitFailed)?;

        Ok(())
    }
}

/// 按键中断处理函数
fn handle_key_interrupt(_: Interrupt) {
    critical_section::with(|cs| {
        let mut state = SHARED_STATE.borrow_ref_mut(cs);
        let Some(state) = &mut *state else {
            return;
        };

        let pin_level = critical_section::with(|_| {
            // 读取引脚电平（需根据实际硬件调整：低电平按下/高电平按下）
            unsafe { &*SHARED_STATE.get() }
                .as_ref()
                .map(|s| s.timer.count())
                .unwrap_or(0)
        });

        if pin_level == 0 {
            // 按键按下：记录开始时间
            state.press_start_time = Some(esp_hal::time::now().as_millis() as u64);
            state.timer.start(LONG_PRESS_THRESHOLD_MS);
        } else {
            // 按键松开：判断长短按
            if let Some(start_time) = state.press_start_time.take() {
                let press_duration = esp_hal::time::now().as_millis() as u64 - start_time;
                if press_duration >= LONG_PRESS_THRESHOLD_MS {
                    let _ = state.long_press_sig.signal(KeyEvent::LongPress);
                } else {
                    let _ = state.short_press_sig.signal(KeyEvent::ShortPress);
                }
            }
            state.timer.stop();
        }
    });
}
