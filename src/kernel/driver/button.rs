// src/kernel/driver/button.rs
//! 按键驱动模块
//!
//! 提供ESP32C平台的按键驱动功能，支持中断方式按键检测、防抖和长按识别

use crate::common::error::Result;
use core::sync::atomic::{AtomicBool, Ordering};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;

/// 按键事件枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonEvent {
    /// 单次按下
    Press,
    /// 长按（持续按下超过指定时间）
    LongPress,
    /// 长按15秒（强制配对模式）
    LongPress15s,
    /// 释放
    Release,
}

/// 按键驱动trait
pub trait ButtonDriver {
    /// 获取按键事件
    async fn get_event(&mut self) -> Result<ButtonEvent>;

    /// 检查按键是否按下
    fn is_pressed(&self) -> bool;
}

/// 按键配置
#[derive(Debug, Clone, Copy)]
pub struct ButtonConfig {
    /// 防抖时间（毫秒）
    pub debounce_ms: u32,
    /// 长按阈值（毫秒）
    pub long_press_ms: u32,
}

impl Default for ButtonConfig {
    fn default() -> Self {
        Self {
            debounce_ms: 50,
            long_press_ms: 1000,
        }
    }
}

// 静态变量用于中断处理
static BUTTON_PRESSED: AtomicBool = AtomicBool::new(false);
static BUTTON_EVENT_CHANNEL: Channel<CriticalSectionRawMutex, ButtonEvent, 8> = Channel::new();

/// ESP32C6按键驱动实现
#[cfg(feature = "embedded_esp")]
pub struct EspButtonDriver {}

#[cfg(feature = "embedded_esp")]
impl EspButtonDriver {
    /// 创建新的按键驱动实例
    pub fn new(peripherals: &esp_hal::peripherals::Peripherals) -> Result<Self> {
        log::info!("Initializing ESP button driver on GPIO9");

        // 配置GPIO9为输入，外部上拉
        // let button_pin = Input::new(
        //     unsafe { peripherals.GPIO9.clone_unchecked() },
        //     InputConfig::default().with_pull(Pull::Up),
        // );

        // 配置中断
        // button_pin.set_interrupt_type(InterruptType::FallingEdge)?;
        // button_pin.enable_interrupt()?;

        // 注册中断处理函数
        // esp_hal::interrupt::enable(
        //     esp_hal::peripherals::Interrupt::GPIO,
        //     esp_hal::interrupt::Priority::Priority3,
        // )?;

        // 启动按键处理任务
        // esp_rtos::task::spawn_pinned(1, Self::button_task, config).map_err(|e| {
        //     log::error!("Failed to spawn button task: {:?}", e);
        //     AppError::SensorError
        // })?;

        Ok(Self {})
    }

    // /// 按键处理任务
    // #[esp_rtos::task]
    // async fn button_task(config: ButtonConfig) {
    //     let mut last_state = true;
    //     let mut press_start_time: Option<u64> = None;

    //     loop {
    //         // 检查按键状态
    //         let current_state = BUTTON_PRESSED.load(Ordering::SeqCst);

    //         if current_state != last_state {
    //             // 防抖处理
    //             esp_rtos::time::sleep(Rate::from_millis(config.debounce_ms)).await;

    //             let debounced_state = BUTTON_PRESSED.load(Ordering::SeqCst);

    //             if debounced_state != last_state {
    //                 last_state = debounced_state;

    //                 if debounced_state {
    //                     // 按键按下
    //                     press_start_time = Some(esp_rtos::time::now().as_millis() as u64);
    //                     let _ = BUTTON_EVENT_CHANNEL.send(ButtonEvent::Press).await;
    //                 } else {
    //                     // 按键释放
    //                     if let Some(start_time) = press_start_time {
    //                         let press_duration =
    //                             esp_rtos::time::now().as_millis() as u64 - start_time;

    //                         if press_duration >= 15000 {
    //                             let _ = BUTTON_EVENT_CHANNEL.send(ButtonEvent::LongPress15s).await;
    //                         } else if press_duration >= config.long_press_ms as u64 {
    //                             let _ = BUTTON_EVENT_CHANNEL.send(ButtonEvent::LongPress).await;
    //                         }

    //                         press_start_time = None;
    //                     }

    //                     let _ = BUTTON_EVENT_CHANNEL.send(ButtonEvent::Release).await;
    //                 }
    //             }
    //         }

    //         // 检查长按
    //         if let Some(start_time) = press_start_time {
    //             let press_duration = esp_rtos::time::now().as_millis() as u64 - start_time;

    //             if press_duration >= 15000 || press_duration >= config.long_press_ms as u64 {
    //                 // 已达到长按阈值，清除开始时间避免重复触发
    //                 press_start_time = None;
    //             }
    //         }

    //         esp_rtos::time::sleep(Rate::from_millis(10)).await;
    //     }
    // }
}

/// GPIO中断处理函数
// #[cfg(feature = "embedded_esp")]
// fn gpio_interrupt_handler() {
//     let peripherals = unsafe { Peripherals::steal() };
//     let gpio = &peripherals.GPIO;

//     // 检查GPIO9的中断标志
//     if gpio.status.read().status9().bit_is_set() {
//         // 清除中断标志
//         gpio.status.write(|w| w.status9().clear_bit());

//         // 设置按键按下标志
//         BUTTON_PRESSED.store(true, Ordering::SeqCst);
//     }
// }

#[cfg(feature = "embedded_esp")]
impl ButtonDriver for EspButtonDriver {
    async fn get_event(&mut self) -> Result<ButtonEvent> {
        // BUTTON_EVENT_CHANNEL.recv().await.map_err(|e| {
        //     log::error!("Failed to receive button event: {:?}", e);
        //     AppError::SensorError
        // })
        unimplemented!()
    }

    fn is_pressed(&self) -> bool {
        BUTTON_PRESSED.load(Ordering::SeqCst)
    }
}

/// 模拟按键驱动（用于Linux和模拟器）
#[cfg(any(feature = "simulator", feature = "embedded_linux"))]
pub struct MockButtonDriver;

#[cfg(any(feature = "simulator", feature = "embedded_linux"))]
impl MockButtonDriver {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(any(feature = "simulator", feature = "embedded_linux"))]
impl ButtonDriver for MockButtonDriver {
    async fn get_event(&mut self) -> Result<ButtonEvent> {
        unimplemented!()
    }

    fn is_pressed(&self) -> bool {
        false
    }
}

/// 默认按键驱动类型别名
#[cfg(feature = "embedded_esp")]
pub type DefaultButtonDriver = EspButtonDriver;

#[cfg(any(feature = "simulator", feature = "embedded_linux"))]
pub type DefaultButtonDriver = MockButtonDriver;
