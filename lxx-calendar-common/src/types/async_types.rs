use embassy_sync::blocking_mutex::raw::RawMutex;
use embassy_sync::channel::{Receiver, Sender};
use embassy_sync::mutex::Mutex;
use embassy_sync::rwlock::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use embassy_sync::signal::Signal;

const CAP: usize = 10;

/// 通道类型
pub type LxxChannel<M: LxxAsyncRawMutex, T> = embassy_sync::channel::Channel<M, T, CAP>;

/// 通道接收者类型
pub type LxxChannelReceiver<M: LxxAsyncRawMutex, T> = Receiver<M, T, CAP>;

/// 通道发送者类型
pub type LxxChannelSender<M: LxxAsyncRawMutex, T> = Sender<M, T, CAP>;

/// 异步原始互斥锁 trait
pub trait LxxAsyncRawMutex: RawMutex + Send + Sync {}

/// 异步互斥锁策略 trait
/// 用于定义平台特定的异步互斥锁行为
pub trait LxxAsyncMutexStrategy {
    /// 获取互斥锁的方法
    async fn lock<R>(&self, f: impl FnOnce() -> R) -> R;
}

/// 同步互斥锁策略 trait
/// 用于定义平台特定的同步互斥锁行为
pub trait LxxSyncMutexStrategy {
    /// 获取互斥锁的方法
    fn lock<R>(&self, f: impl FnOnce() -> R) -> R;
}
