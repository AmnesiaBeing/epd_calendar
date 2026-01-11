
use embassy_sync::blocking_mutex::raw::RawMutex as EmbassyRawMutex;
use embassy_sync::channel::{Receiver, Sender};
use embassy_sync::mutex::{Mutex};
use embassy_sync::rwlock::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use embassy_sync::signal::Signal;

/// 通道类型
pub type Channel<M: AsyncRawMutex, T, const CAP: usize> = embassy_sync::channel::Channel<M, T, CAP>;

/// 通道接收者类型
pub type ChannelReceiver<'a, M: AsyncRawMutex, T, const CAP: usize> = Receiver<'a, M, T, CAP>;

/// 通道发送者类型
pub type ChannelSender<'a, M: AsyncRawMutex, T, const CAP: usize> = Sender<'a, M, T, CAP>;

/// 异步原始互斥锁 trait
pub trait AsyncRawMutex: EmbassyRawMutex + Send + Sync {}

/// 异步互斥锁策略 trait
/// 用于定义平台特定的异步互斥锁行为
pub trait AsyncMutexStrategy {
    /// 获取互斥锁的方法
    async fn lock<R>(&self, f: impl FnOnce() -> R) -> R;
}

/// 同步互斥锁策略 trait
/// 用于定义平台特定的同步互斥锁行为
pub trait SyncMutexStrategy {
    /// 获取互斥锁的方法
    fn lock<R>(&self, f: impl FnOnce() -> R) -> R;
}

/// 全局互斥锁类型别名
pub type GlobalMutex<T, M: AsyncRawMutex> = Mutex<M, T>;

/// 全局通道类型别名
/// 通道容量固定为32个元素
pub type GlobalChannel<T, M: AsyncRawMutex> = Channel<M, T, 32>;

/// 全局同步读写锁类型别名
pub type GlobalRwLock<T, M: AsyncRawMutex> = RwLock<M, T>;

/// 全局同步读写锁读守卫类型别名
pub type GlobalRwLockReadGuard<'a, T, M: AsyncRawMutex> = RwLockReadGuard<'a, M, T>;

/// 全局同步读写锁写守卫类型别名
pub type GlobalRwLockWriteGuard<'a, T, M: AsyncRawMutex> = RwLockWriteGuard<'a, M, T>;

/// 全局信号类型别名
pub type GlobalSignal<T, M: AsyncRawMutex> = Signal<GlobalMutex<T, M>, T>;