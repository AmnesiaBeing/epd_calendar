use crate::types::async_types::{AsyncRawMutex, AsyncMutexStrategy, SyncMutexStrategy};

/// 硬件资源接口
pub trait RawHwiResource: Sized + Copy + Send + Sync {}

/// 硬件上下文
type HwiContext = ();

/// 平台特定的异步类型集
pub trait PlatformAsyncTypes {
    /// 平台特定的原始互斥锁类型
    type RawMutex: AsyncRawMutex;
    
    /// 平台特定的互斥锁类型
    type Mutex<T>: Send + Sync
    where
        T: Send + Sync;
    
    /// 平台特定的互斥锁守卫类型
    type MutexGuard<'a, T>: Send
    where
        T: Send + Sync;
    
    /// 平台特定的通道类型
    type Channel<T, const CAP: usize>: Send + Sync
    where
        T: Send + Sync;
    
    /// 平台特定的读写锁类型
    type RwLock<T>: Send + Sync
    where
        T: Send + Sync;
    
    /// 平台特定的读写锁读守卫类型
    type RwLockReadGuard<'a, T>: Send
    where
        T: Send + Sync;
    
    /// 平台特定的读写锁写守卫类型
    type RwLockWriteGuard<'a, T>: Send
    where
        T: Send + Sync;
    
    /// 平台特定的信号类型
    type Signal<T>: Send + Sync
    where
        T: Send + Sync;
}

pub trait Platform: Sized {
    /// 平台特定的异步类型集
    type AsyncTypes: PlatformAsyncTypes;
    
    type EventBusPubSubMutexType: AsyncRawMutex;

    type WatchDogMutexStrategy: AsyncMutexStrategy;

    type EventBusMutexStrategy: AsyncMutexStrategy;

    const SPI_FREQUENCY: u32;
    type SpiController: AsyncMutexStrategy;

    const I2C_FREQUENCY: u32;
    type I2cMotionMutexStrategy: AsyncMutexStrategy;

    type PSOnMutexStrategy: SyncMutexStrategy;

    type ProbePwm: SyncMutexStrategy;
    type ProbePwmChannel: RawHwiResource + Copy;

    /// Initialize the logger (optional)
    fn init_logger() {}

    /// Initialize the heap allocator (if needed)
    fn init_heap() {}

    // The HWI initialization
    fn init(
        spawner: embassy_executor::Spawner,
    ) -> impl core::future::Future<Output = HwiContext>;

    /// Resets the MCU/CPU
    fn sys_reset();

    /// Stop the MCU/CPU
    fn sys_stop();
}