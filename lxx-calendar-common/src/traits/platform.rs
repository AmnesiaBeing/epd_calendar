use core::marker::PhantomData;

pub trait PlatformTrait: Sized {
    /// 初始化日志
    fn init_logger() {}

    /// 初始化堆栈
    fn init_heap() {}

    // 硬件实现初始化
    fn init(
        spawner: embassy_executor::Spawner,
    ) -> impl core::future::Future<Output = PlatformContext<Self>>;

    /// 复位处理器
    fn sys_reset();

    /// 停止处理器，进入深度休眠
    fn sys_stop();

    type StaticWatchDogControllerMutexType;

    type StatiEpdControllerMutexType;
}

pub struct PlatformContext<C>
where
    C: PlatformTrait + Sized,
{
    pub sys_watch_dog: C::StaticWatchDogControllerMutexType,
    pub epd: C::StatiEpdControllerMutexType,
}
