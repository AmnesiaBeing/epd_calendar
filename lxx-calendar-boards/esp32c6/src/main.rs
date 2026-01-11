#![no_std]
#![no_main]

pub use lxx_calendar_core as lxx_core;
use lxx_calendar_common as lxxcc;
use lxx_core::traits::Platform;
use lxx_core::traits::PlatformAsyncTypes;
use lxx_core::types::async_types::{Mutex, MutexGuard, Channel, RwLock, RwLockReadGuard, RwLockWriteGuard, Signal};

use embassy_executor::Spawner;
use esp_rtos::main as platform_main;
use esp_sync::RawMutex;

#[platform_main]
async fn main(spawner: Spawner) {
    // 调用核心应用逻辑，不需要传递系统事件通道，由core_main内部创建
    lxx_core::core_main::<RawMutex>(spawner).await.unwrap();
}

/// ESP32C6平台的异步类型实现
pub struct Esp32AsyncTypes;
impl PlatformAsyncTypes for Esp32AsyncTypes {
    /// 使用ESP32的RawMutex作为原始互斥锁
    type RawMutex = RawMutex;
    
    /// 使用embassy_sync的Mutex
    type Mutex<T> = Mutex<RawMutex, T>;
    
    /// 使用embassy_sync的MutexGuard
    type MutexGuard<'a, T> = MutexGuard<'a, RawMutex, T>;
    
    /// 使用embassy_sync的Channel
    type Channel<T, const CAP: usize> = Channel<RawMutex, T, CAP>;
    
    /// 使用embassy_sync的RwLock
    type RwLock<T> = RwLock<RawMutex, T>;
    
    /// 使用embassy_sync的RwLockReadGuard
    type RwLockReadGuard<'a, T> = RwLockReadGuard<'a, RawMutex, T>;
    
    /// 使用embassy_sync的RwLockWriteGuard
    type RwLockWriteGuard<'a, T> = RwLockWriteGuard<'a, RawMutex, T>;
    
    /// 使用embassy_sync的Signal
    type Signal<T> = Signal<Mutex<RawMutex, T>, T>;
}

/// ESP32C6平台实现
pub struct Esp32Platform;
impl Platform for Esp32Platform {
    /// 使用ESP32C6的异步类型集
    type AsyncTypes = Esp32AsyncTypes;
    
    type EventBusPubSubMutexType = RawMutex;
    
    type WatchDogMutexStrategy = Self::SpiController;
    
    type EventBusMutexStrategy = Self::SpiController;
    
    const SPI_FREQUENCY: u32 = 20_000_000;
    type SpiController = Esp32AsyncTypes;
    
    const I2C_FREQUENCY: u32 = 400_000;
    type I2cMotionMutexStrategy = Self::SpiController;
    
    type PSOnMutexStrategy = Self::SpiController;
    
    type ProbePwm = Self::SpiController;
    type ProbePwmChannel = ();
    
    async fn init(spawner: embassy_executor::Spawner) -> lxx_calendar_common::traits::HwiContext {
        ()
    }
    
    fn sys_reset() {
        // ESP32C6的系统重置实现
        unsafe {
            esp_riscv_rt::reset();
        }
    }
    
    fn sys_stop() {
        // ESP32C6的系统停止实现
        unsafe {
            esp_riscv_rt::wfi();
        }
    }
}