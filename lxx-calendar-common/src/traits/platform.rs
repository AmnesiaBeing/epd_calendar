pub trait Platform: Sized {
    /// 平台特定的异步类型集
    type AsyncTypes: PlatformAsyncTypes;

    type EventBusPubSubMutexType: AsyncRawMutex;

    type WatchDogMutexStrategy: AsyncMutexStrategy;

    type EventBusMutexStrategy: AsyncMutexStrategy;

    type EpdSpiController: AsyncMutexStrategy;

    type SensorI2cMutexStrategy: AsyncMutexStrategy;

    type BuzzerPwm: SyncMutexStrategy;
    type BuzzerPwmChannel: RawHwiResource + Copy;

    /// 初始化日志
    fn init_logger() {}

    /// 初始化堆栈
    fn init_heap() {}

    // 硬件实现初始化
    fn init(spawner: embassy_executor::Spawner) -> impl core::future::Future<Output = HwiContext>;

    /// 复位处理器
    fn sys_reset();

    /// 停止处理器，进入深度休眠
    fn sys_stop();
}
