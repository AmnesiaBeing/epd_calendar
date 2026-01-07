// src/common/mod.rs

pub mod error;

use embassy_sync_07::rwlock::RwLockWriteGuard;
use embassy_sync_07::rwlock::{RwLock, RwLockReadGuard};
use embassy_sync_07::signal::Signal;
use embassy_sync_07::{channel::Channel, mutex::Mutex};

#[cfg(any(feature = "simulator", feature = "tspi"))]
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
#[cfg(feature = "esp32")]
use esp_sync::RawMutex;

/// 全局互斥锁类型别名
///
/// 根据目标平台选择不同的互斥锁实现
/// - 模拟器和嵌入式Linux：使用ThreadModeRawMutex
/// - 嵌入式ESP平台：使用ESP的RawMutex
#[cfg(any(feature = "simulator", feature = "tspi"))]
pub type GlobalMutex<T> = Mutex<ThreadModeRawMutex, T>;
#[cfg(feature = "esp32")]
pub type GlobalMutex<T> = Mutex<RawMutex, T>;

/// 全局通道类型别名
///
/// 根据目标平台选择不同的通道实现
/// - 模拟器和嵌入式Linux：使用ThreadModeRawMutex
/// - 嵌入式ESP平台：使用ESP的RawMutex
/// 通道容量固定为32个元素
#[cfg(any(feature = "simulator", feature = "tspi"))]
pub type GlobalChannel<T> = Channel<ThreadModeRawMutex, T, 32>;
#[cfg(feature = "esp32")]
pub type GlobalChannel<T> = Channel<RawMutex, T, 32>;

/// 全局同步读写锁类型别名
///
/// 根据目标平台选择不同的读写锁实现
/// - 模拟器和嵌入式Linux：使用ThreadModeRawMutex
/// - 嵌入式ESP平台：使用ESP的RawMutex
/// 通道容量固定为32个元素
#[cfg(any(feature = "simulator", feature = "tspi"))]
pub type GlobalRwLock<T> = RwLock<ThreadModeRawMutex, T>;
#[cfg(feature = "esp32")]
pub type GlobalRwLock<T> = RwLock<RawMutex, T>;

/// 全局同步读写锁读守卫类型别名
///
/// 根据目标平台选择不同的读写锁读守卫实现
/// - 模拟器和嵌入式Linux：使用ThreadModeRawMutex
/// - 嵌入式ESP平台：使用ESP的RawMutex
/// 通道容量固定为32个元素
#[cfg(any(feature = "simulator", feature = "tspi"))]
pub type GlobalRwLockReadGuard<'a, T> = RwLockReadGuard<'a, ThreadModeRawMutex, T>;
#[cfg(feature = "esp32")]
pub type GlobalRwLockReadGuard<'a, T> = RwLockReadGuard<'a, RawMutex, T>;

/// 全局同步读写锁写    守卫类型别名
///
/// 根据目标平台选择不同的读写锁守卫实现
/// - 模拟器和嵌入式Linux：使用ThreadModeRawMutex
/// - 嵌入式ESP平台：使用ESP的RawMutex
/// 通道容量固定为32个元素
#[cfg(any(feature = "simulator", feature = "tspi"))]
pub type GlobalRwLockWriteGuard<'a, T> = RwLockWriteGuard<'a, ThreadModeRawMutex, T>;
#[cfg(feature = "esp32")]
pub type GlobalRwLockWriteGuard<'a, T> = RwLockWriteGuard<'a, RawMutex, T>;

/// 全局信号类型别名
pub type GlobalSignal<T> = Signal<GlobalMutex<T>, T>;
